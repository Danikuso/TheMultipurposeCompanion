use crate::*;
use chrono::{DateTime, Duration, Utc};
use flate2::{Compression, write::ZlibEncoder};
use std::io::Write;
use tmc_model::market::{MarketSnapshot, PriceOrigin};
use tmc_model::net::PadSize;

const COMMODITY: &str = include_str!("../../../tests/fixtures/eddn/commodity_ok.json");
const DOCKED: &str = include_str!("../../../tests/fixtures/eddn/journal_docked.json");

fn z(s: &str) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(s.as_bytes()).unwrap();
    e.finish().unwrap()
}

fn now() -> DateTime<Utc> {
    "2026-01-01T12:05:00Z".parse().unwrap()
}

fn commodity_at(ts: &str, market_id: u64, station: &str) -> String {
    format!(
        r#"{{"$schemaRef":"{COMMODITY_SCHEMA}","header":{{}},"message":{{"timestamp":"{ts}","systemName":"Lave","stationName":"{station}","marketId":{market_id},"commodities":[]}}}}"#
    )
}

fn snap(system: &str) -> MarketSnapshot {
    match decode(&z(&COMMODITY.replace("Sistema Ficticio", system)), now()) {
        Decoded::Market(m) => m,
        other => panic!("{other:?}"),
    }
}

#[test]
fn decodes_commodity_fixture() {
    let Decoded::Market(m) = decode(&z(COMMODITY), now()) else {
        panic!("no es Market");
    };
    assert_eq!(m.prices.len(), 2);
    assert_eq!(m.prices[0].commodity, "explosives");
    assert_eq!(m.origin, PriceOrigin::Eddn);
    assert_eq!(m.market_id, 128000001);
}

#[test]
fn corrupt_zlib_is_dropped_and_counted() {
    assert_eq!(
        decode(b"\x78\x9cbasura", now()),
        Decoded::Ignored(DropReason::Corrupt)
    );
    let full = z(COMMODITY);
    assert_eq!(
        decode(&full[..full.len() / 2], now()),
        Decoded::Ignored(DropReason::Corrupt)
    );
    assert_eq!(
        decode(&z("{no json"), now()),
        Decoded::Ignored(DropReason::Corrupt)
    );
    assert_eq!(
        decode(
            &z(&commodity_at("2026-01-01T12:00:00Z", 1, "A")
                .replace("\"marketId\":1", "\"marketId\":\"x\"")),
            now()
        ),
        Decoded::Ignored(DropReason::Corrupt)
    );
}

#[test]
fn oversized_payload_is_dropped() {
    let big = " ".repeat(2 << 20);
    assert_eq!(
        decode(&z(&big), now()),
        Decoded::Ignored(DropReason::TooLarge)
    );
}

#[test]
fn journal_fsdjump_yields_coords_only() {
    let j = format!(
        r#"{{"$schemaRef":"{JOURNAL_SCHEMA}","header":{{}},"message":{{"timestamp":"2026-01-01T12:00:00Z","event":"FSDJump","StarSystem":"Lave","SystemAddress":42,"StarPos":[75.75,48.75,70.75]}}}}"#
    );
    assert_eq!(
        decode(&z(&j), now()),
        Decoded::Coords("lave".into(), Some(42), [75.75, 48.75, 70.75])
    );
}

#[test]
fn test_schema_and_unknown_schema_are_dropped() {
    let t = COMMODITY.replace("commodity/3", "commodity/3/test");
    assert_eq!(
        decode(&z(&t), now()),
        Decoded::Ignored(DropReason::TestSchema)
    );
    let o = COMMODITY.replace("commodity/3", "outfitting/2");
    assert_eq!(decode(&z(&o), now()), Decoded::Ignored(DropReason::Schema));
}

#[test]
fn stale_and_future_timestamps_dropped() {
    let n = now();
    let old = (n - Duration::days(8)).to_rfc3339();
    let fut = (n + Duration::minutes(10)).to_rfc3339();
    assert_eq!(
        decode(&z(&commodity_at(&old, 128000001, "Puerto")), n),
        Decoded::Ignored(DropReason::Stale)
    );
    assert_eq!(
        decode(&z(&commodity_at(&fut, 128000001, "Puerto")), n),
        Decoded::Ignored(DropReason::Future)
    );
}

#[test]
fn carrier_market_dropped() {
    assert_eq!(
        decode(
            &z(&commodity_at(
                "2026-01-01T12:00:00Z",
                3_700_000_123,
                "K7Q-12B"
            )),
            now()
        ),
        Decoded::Ignored(DropReason::Carrier)
    );
    // Un MarketID alto sin indicativo no es carrier, salvo que se recuerde de un Docked.
    let m = commodity_at("2026-01-01T12:00:00Z", 3_700_000_123, "Deposito");
    assert!(matches!(decode(&z(&m), now()), Decoded::Market(_)));
    let mut set = CarrierSet::new();
    set.remember(3_700_000_123);
    assert_eq!(
        decode_with(&z(&m), now(), &mut set),
        Decoded::Ignored(DropReason::Carrier)
    );
}

#[test]
fn journal_docked_feeds_coords_and_pads() {
    let Decoded::Station(info, coords) = decode(&z(DOCKED), now()) else {
        panic!("no es Station");
    };
    assert_eq!(info.max_pad, Some(PadSize::Large));
    assert_eq!(info.market_id, 128000001);
    assert_eq!(info.station_type.as_deref(), Some("Coriolis"));
    assert_eq!(info.star_pos, Some([10.5, -20.25, 30.125]));
    assert_eq!(
        coords,
        Some((
            "sistema ficticio".to_string(),
            Some(1000000000001),
            [10.5, -20.25, 30.125]
        ))
    );
}

#[test]
fn unknown_coords_commodity_dropped_and_counted() {
    let idx = CoordIndex::new();
    let f = EddnFilter {
        enabled: true,
        center: Some([0.0; 3]),
        radius_ly: 200.0,
    };
    let mut s = snap("Sistema Ficticio");
    assert_eq!(accept(&mut s, &idx, &f), Err(DropReason::UnknownCoords));
    let mut stats = EddnStats::default();
    *stats.dropped.entry(DropReason::UnknownCoords).or_default() += 1;
    assert_eq!(stats.dropped[&DropReason::UnknownCoords], 1);
}

#[test]
fn far_market_dropped_near_market_accepted() {
    let mut idx = CoordIndex::new();
    idx.insert("cerca".into(), None, [150.0, 0.0, 0.0], None);
    idx.insert("lejos".into(), None, [250.0, 0.0, 0.0], None);
    let f = EddnFilter {
        enabled: true,
        center: Some([0.0; 3]),
        radius_ly: 200.0,
    };
    let mut near = snap("Cerca");
    assert_eq!(accept(&mut near, &idx, &f), Ok(()));
    assert_eq!(near.star_pos, Some([150.0, 0.0, 0.0]));
    let mut far = snap("Lejos");
    assert_eq!(accept(&mut far, &idx, &f), Err(DropReason::Far));
}

#[test]
fn coord_index_is_bounded() {
    let mut idx = CoordIndex::new();
    let c = Some([0.0; 3]);
    for i in 0..COORD_INDEX_CAP {
        idx.insert(format!("s{i}"), None, [100.0 + i as f64, 0.0, 0.0], c);
    }
    idx.insert("ultima".into(), None, [1.0, 0.0, 0.0], c);
    assert_eq!(idx.len(), COORD_INDEX_CAP);
    assert!(idx.get("ultima").is_some());
    // Se descartó la más lejana.
    assert!(idx.get(&format!("s{}", COORD_INDEX_CAP - 1)).is_none());
}

fn journal(event: &str, ts: &str, extra: &str) -> Vec<u8> {
    z(&format!(
        r#"{{"$schemaRef":"{JOURNAL_SCHEMA}","header":{{}},"message":{{"timestamp":"{ts}","event":"{event}",{extra}}}}}"#
    ))
}

#[test]
fn docked_carrier_is_dropped_and_remembered() {
    let d = DOCKED
        .replace("Coriolis", "FleetCarrier")
        .replace("128000001", "3700000999");
    let mut set = CarrierSet::new();
    assert_eq!(
        decode_with(&z(&d), now(), &mut set),
        Decoded::Ignored(DropReason::Carrier)
    );
    assert!(set.contains(3_700_000_999));
}

#[test]
fn carrier_set_is_bounded() {
    let mut s = CarrierSet::new();
    for i in 0..20_001u64 {
        s.remember(i);
    }
    assert_eq!(s.len(), 20_000);
    assert!(!s.contains(0));
    assert!(s.contains(20_000));
}

#[test]
fn scan_is_schema_and_coords_do_not_go_stale() {
    assert_eq!(
        decode(&journal("Scan", "garbage", r#""StarSystem":"X""#), now()),
        Decoded::Ignored(DropReason::Schema)
    );
    let old = (now() - Duration::days(60)).to_rfc3339();
    let j = journal("FSDJump", &old, r#""StarSystem":"Lave","StarPos":[1,2,3]"#);
    assert!(matches!(decode(&j, now()), Decoded::Coords(..)));
}

#[test]
fn invalid_coords_are_corrupt() {
    for pos in ["[1e9,0,0]", "[0,-300000,0]"] {
        let j = journal(
            "FSDJump",
            "2026-01-01T12:00:00Z",
            &format!(r#""StarSystem":"L","StarPos":{pos}"#),
        );
        assert_eq!(decode(&j, now()), Decoded::Ignored(DropReason::Corrupt));
    }
    let j = journal(
        "FSDJump",
        "2026-01-01T12:00:00Z",
        r#""StarSystem":"L","StarPos":[-5.5,0,7]"#,
    );
    assert!(matches!(decode(&j, now()), Decoded::Coords(..)));
}

#[test]
fn time_boundaries_are_inclusive() {
    let n = now();
    let ok_old = (n - Duration::days(7)).to_rfc3339();
    let ok_fut = (n + Duration::minutes(5)).to_rfc3339();
    assert!(matches!(
        decode(&z(&commodity_at(&ok_old, 128000001, "P")), n),
        Decoded::Market(_)
    ));
    assert!(matches!(
        decode(&z(&commodity_at(&ok_fut, 128000001, "P")), n),
        Decoded::Market(_)
    ));
}

#[test]
fn wrong_structure_is_corrupt() {
    let bad = format!(r#"{{"$schemaRef":"{COMMODITY_SCHEMA}","message":[]}}"#);
    assert_eq!(
        decode(&z(&bad), now()),
        Decoded::Ignored(DropReason::Corrupt)
    );
    let neg = COMMODITY.replace(r#""buyPrice":203"#, r#""buyPrice":-3"#);
    assert_eq!(
        decode(&z(&neg), now()),
        Decoded::Ignored(DropReason::Corrupt)
    );
}

#[test]
fn filter_boundaries_and_fail_closed() {
    let mut idx = CoordIndex::new();
    idx.insert("borde".into(), None, [200.0, 0.0, 0.0], None);
    let mk = |radius_ly, center| EddnFilter {
        enabled: true,
        center,
        radius_ly,
    };
    let mut s = snap("Borde");
    assert_eq!(accept(&mut s, &idx, &mk(200.0, Some([0.0; 3]))), Ok(()));
    assert_eq!(
        accept(&mut s, &idx, &mk(f64::NAN, Some([0.0; 3]))),
        Err(DropReason::Far)
    );
    assert_eq!(
        accept(&mut s, &idx, &mk(-1.0, Some([0.0; 3]))),
        Err(DropReason::Far)
    );
    assert_eq!(accept(&mut s, &idx, &mk(10.0, None)), Ok(()));
}

#[test]
fn full_index_rejects_farther_entry() {
    let mut idx = CoordIndex::new();
    let c = Some([0.0; 3]);
    for i in 0..COORD_INDEX_CAP {
        idx.insert(format!("s{i}"), None, [1.0 + i as f64, 0.0, 0.0], c);
    }
    idx.insert("muylejos".into(), None, [9e9, 0.0, 0.0], c);
    assert_eq!(idx.len(), COORD_INDEX_CAP);
    assert!(idx.get("muylejos").is_none());
}
