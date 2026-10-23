//! Detalles privados y paginas acotadas por bytes, sin cursores compartidos.
use serde_json::{Value, json};
use tmc_core::{ModulePageQuery, ModuleQueryError};

pub(super) const PAGE_LIMIT: usize = 16384;

pub(super) fn page(
    revision: u64,
    section: &str,
    rows: &[Value],
    query: &ModulePageQuery,
) -> Result<Value, ModuleQueryError> {
    if query.revision != revision {
        return Err(ModuleQueryError::StaleRevision);
    }
    if query.cursor > rows.len() {
        return Err(ModuleQueryError::InvalidQuery);
    }
    let wrapper = |items: Vec<Value>, next: Option<usize>| {
        json!({
            "revision": revision, "section": section, "cursor": query.cursor,
            "next_cursor": next, "total": rows.len(), "items": items,
        })
    };
    let mut bytes = 0;
    let mut end = query.cursor;
    for row in &rows[query.cursor..] {
        let size = serde_json::to_vec(row).unwrap().len() + usize::from(end > query.cursor);
        let candidate_end = end + 1;
        let next = (candidate_end < rows.len()).then_some(candidate_end);
        let overhead = serde_json::to_vec(&wrapper(Vec::new(), next))
            .unwrap()
            .len();
        if overhead + bytes + size >= PAGE_LIMIT {
            break;
        }
        bytes += size;
        end = candidate_end;
    }
    if end == query.cursor && end < rows.len() {
        return Err(ModuleQueryError::ItemTooLarge);
    }
    Ok(wrapper(
        rows[query.cursor..end].to_vec(),
        (end < rows.len()).then_some(end),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_page_uses_actual_wrapper_at_exact_byte_boundary() {
        let query = ModulePageQuery {
            revision: 42,
            cursor: 0,
            section: None,
            engineer_id: None,
        };
        for name in ["a".repeat(16258), "\u{e9}".repeat(8129)] {
            let row = json!({"count":1,"key":"x","name_localised":name});
            let expected = json!({"revision":42,"section":"items","cursor":0,"next_cursor":null,"total":1,"items":[row.clone()]});
            assert_eq!(serde_json::to_vec(&expected).unwrap().len(), 16383);
            let actual = page(42, "items", &[row], &query).unwrap();
            assert_eq!(actual, expected);
            let mut oversized = actual["items"][0].clone();
            oversized["name_localised"] = json!(format!(
                "{}a",
                oversized["name_localised"].as_str().unwrap()
            ));
            let mut expected_oversized = expected;
            expected_oversized["items"][0] = oversized.clone();
            assert_eq!(
                serde_json::to_vec(&expected_oversized).unwrap().len(),
                16384
            );
            assert_eq!(
                page(42, "items", &[oversized], &query),
                Err(ModuleQueryError::ItemTooLarge)
            );
        }
    }
}
