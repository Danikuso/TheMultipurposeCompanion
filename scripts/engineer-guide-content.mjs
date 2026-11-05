// Redacción propia TMC; hechos de la base Apache-2.0 y correcciones documentadas en F16 T0.
// Se aplica al renderizar el catálogo para conservar la importación determinista del snapshot.
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const langs = ['es', 'en', 'fr', 'de', 'pt'];
const words = text => text.split('|');
const source = 'https://github.com/EDDiscovery/EliteDangerousCore/blob/7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7/EliteDangerous/FrontierData/Items/Engineers.cs';
const update = 'https://store.steampowered.com/news/posts/?appids=359320&enddate=1726826562&feed=steam_community_announcements';
const parents = {
  300000: [300210], 300010: [300210], 300030: [300090], 300040: [300250],
  300050: [300160], 300090: [300080], 300110: [300120], 300120: [300180],
  300130: [300260], 300140: [300250], 300150: [300180], 300200: [300160],
  300210: [300260], 300220: [300200], 300230: [300200], 300250: [300100],
  300270: [300090], 300280: [300160], 300290: [300080], 300300: [300200],
  400004: [400002], 400005: [400003], 400006: [400001], 400007: [400005],
  400008: [400006], 400009: [400004], 400013: [400010, 400011, 400012],
};
// Nombres canónicos ingleses de objetos: identificables también en guías y herramientas del juego.
const donations = {
  300000: ['lavianbrandy', 'commodity', 50, 'Lavian Brandy'],
  300010: ['bromellite', 'commodity', 50, 'Bromellite'],
  300030: ['fujintea', 'commodity', 50, 'Fujin Tea'],
  300040: ['shieldpatternanalysis', 'material', 50, 'Aberrant Shield Pattern Analysis'],
  300050: ['xihecompanions', 'commodity', 25, 'Xihe Companions'],
  300080: ['landmines', 'commodity', 200, 'Landmines'],
  300090: ['kamitracigars', 'commodity', 50, 'Kamitra Cigars'],
  300100: ['metaalloys', 'commodity', 1, 'Meta-Alloys'],
  300110: ['scandatabanks', 'material', 50, 'Classified Scan Databanks'],
  300120: ['gold', 'commodity', 200, 'Gold'],
  300130: ['progenitorcells', 'commodity', 200, 'Progenitor Cells'],
  300160: ['soontillrelics', 'commodity', 3, 'Soontill Relics'],
  300180: ['credits', 'credits', 500000, 'CR'],
  300200: ['modularterminals', 'commodity', 25, 'Modular Terminals'],
  300220: ['unknownenergysource', 'material', 25, 'Sensor Fragments'],
  300230: ['konggaale', 'commodity', 25, 'Kongga Ale'],
  300270: ['decodedemissiondata', 'material', 50, 'Decoded Emission Data'],
  300290: ['occupiedcryopod', 'commodity', 25, 'Occupied Escape Pods'],
  300300: ['unknownenergysource', 'material', 25, 'Sensor Fragments'],
};
const referrals = {
  400001: [5, 'Genetic Repair Meds', 'Terra Velasquez'],
  400002: [5, 'Push', 'Kit Fowler'],
  400003: [5, 'Settlement Defence Plans', 'Wellington Beck'],
  400004: [5, 'Surveillance Equipment', 'Yarden Bond'],
  400005: [5, 'Insight Entertainment Suites', 'Uma Laszlo'],
  400006: [15, 'Financial Projections', 'Oden Geiger'],
  400010: [10, 'Faction Associates', 'Yi Shen'],
  400011: [10, 'Digital Designs', 'Yi Shen'],
  400012: [10, 'Manufacturing Instructions', 'Yi Shen'],
};
const meeting = {};
const set = (id, text) => { meeting[id] = words(text); };
function rank(id, type, rank) {
  const kind = { trade: words('comercio|trade|commerce|Handel|comércio'), combat: words('combate|combat|combat|Kampf|combate'), exploration: words('exploración|exploration|exploration|Erkundung|exploração') }[type];
  meeting[id] = [
    `Alcanza el rango ${rank} o superior en ${kind[0]}.`, `Reach ${kind[1]} rank ${rank} or higher.`,
    `Atteignez le rang ${rank} ou supérieur en ${kind[2]}.`, `Erreiche im Bereich ${kind[3]} mindestens Rang ${rank}.`,
    `Alcança o nível ${rank} ou superior em ${kind[4]}.`,
  ];
}
rank(300000, 'trade', 'Merchant'); rank(300030, 'combat', 'Competent'); rank(300100, 'exploration', 'Scout');
rank(300110, 'exploration', 'Surveyor'); rank(300130, 'combat', 'Expert'); rank(300150, 'exploration', 'Surveyor');
rank(300290, 'trade', 'Dealer');
set(300010, 'Consigue reputación Friendly con la Alianza y Allied con Alioth Independents; obtén el permiso de Alioth.|Reach Friendly with the Alliance and Allied with Alioth Independents; obtain the Alioth permit.|Atteignez Friendly avec l’Alliance et Allied avec Alioth Independents ; obtenez le permis d’Alioth.|Erreiche Friendly bei der Allianz und Allied bei Alioth Independents; besorge die Alioth-Genehmigung.|Alcança Friendly com a Aliança e Allied com Alioth Independents; obtém a licença de Alioth.');
set(300040, 'Alcanza el rango naval federal Midshipman o superior.|Reach Federal Navy rank Midshipman or higher.|Atteignez le rang naval fédéral Midshipman ou supérieur.|Erreiche mindestens den Föderations-Marinerang Midshipman.|Alcança o nível naval federal Midshipman ou superior.');
set(300050, 'Consigue la invitación de la facción Party of Yoru mediante sus misiones y reputación.|Obtain the Party of Yoru faction invitation through its missions and reputation.|Obtenez l’invitation de la faction Party of Yoru grâce à ses missions et à votre réputation.|Erhalte die Einladung der Fraktion Party of Yoru über ihre Missionen und deinen Ruf.|Obtém o convite da facção Party of Yoru através das suas missões e reputação.');
set(300080, 'Alcanza reputación Cordial o Friendly con Eurybia Blue Mafia.|Reach Cordial or Friendly reputation with Eurybia Blue Mafia.|Atteignez une réputation Cordial ou Friendly avec Eurybia Blue Mafia.|Erreiche den Ruf Cordial oder Friendly bei Eurybia Blue Mafia.|Alcança reputação Cordial ou Friendly com Eurybia Blue Mafia.');
set(300090, 'Alcanza el rango naval imperial Outsider o superior.|Reach Imperial Navy rank Outsider or higher.|Atteignez le rang naval impérial Outsider ou supérieur.|Erreiche mindestens den imperialen Marinerang Outsider.|Alcança o nível naval imperial Outsider ou superior.');
set(300120, 'Comercia en más de 50 mercados diferentes.|Trade at more than 50 different markets.|Commercez dans plus de 50 marchés différents.|Handle auf mehr als 50 verschiedenen Märkten.|Comercia em mais de 50 mercados diferentes.');
set(300140, 'Alcanza reputación Friendly con la Federación y consigue el permiso de Sol.|Reach Friendly with the Federation and obtain the Sol permit.|Atteignez Friendly avec la Fédération et obtenez le permis de Sol.|Erreiche Friendly bei der Föderation und besorge die Sol-Genehmigung.|Alcança Friendly com a Federação e obtém a licença de Sol.');
for(const [id, distance] of [[300160,300],[300220,5000],[300300,5000]]) meeting[id]=[
  `Aléjate al menos ${distance} AL del sistema donde comenzaste tu carrera; no es distancia acumulada.`,
  `Travel at least ${distance} ly from your career starting system; cumulative travel does not count.`,
  `Éloignez-vous d’au moins ${distance} AL de votre système de départ ; la distance cumulée ne compte pas.`,
  `Entferne dich mindestens ${distance} Lj vom Startsystem deiner Karriere; die Gesamtflugstrecke zählt nicht.`,
  `Afasta-te pelo menos ${distance} AL do sistema onde iniciaste a carreira; a distância acumulada não conta.`,
];
set(300180, 'Vende mercancías en al menos 5 mercados negros diferentes.|Sell goods at at least 5 different black markets.|Vendez des marchandises dans au moins 5 marchés noirs différents.|Verkaufe Waren auf mindestens 5 verschiedenen Schwarzmärkten.|Vende mercadorias em pelo menos 5 mercados negros diferentes.');
set(300200, 'Alíate con Sirius Corporation, consigue el permiso de Sirius y completa su misión de invitación al ingeniero.|Become Allied with Sirius Corporation, obtain the Sirius permit and complete its engineer invitation mission.|Devenez Allied avec Sirius Corporation, obtenez le permis de Sirius et accomplissez sa mission d’invitation à l’ingénieur.|Werde Allied bei Sirius Corporation, besorge die Sirius-Genehmigung und erledige ihre Ingenieur-Einladungsmission.|Torna-te Allied com Sirius Corporation, obtém a licença de Sirius e conclui a missão de convite ao engenheiro.');
set(300210, 'Extrae al menos 500 toneladas de mineral con minería; comprarlo no cuenta.|Mine at least 500 tonnes of ore; buying it does not count.|Extrayez au moins 500 tonnes de minerai ; en acheter ne compte pas.|Baue mindestens 500 Tonnen Erz ab; Kaufen zählt nicht.|Extrai pelo menos 500 toneladas de minério; comprá-lo não conta.');
set(300230, 'Alcanza rango de combate Dangerous o superior y consigue el permiso de Shinrarta Dezhra (normalmente con tu primer Elite).|Reach combat rank Dangerous or higher and obtain the Shinrarta Dezhra permit (normally through your first Elite rank).|Atteignez Dangerous en combat ou plus et obtenez le permis de Shinrarta Dezhra (normalement avec votre premier rang Elite).|Erreiche mindestens Kampfrang Dangerous und besorge die Shinrarta-Dezhra-Genehmigung (normalerweise mit deinem ersten Elite-Rang).|Alcança Dangerous ou superior em combate e obtém a licença de Shinrarta Dezhra (normalmente com o primeiro nível Elite).');
set(300250, 'Consigue más de 50 bonos de combate en zonas de conflicto.|Earn more than 50 combat bonds in conflict zones.|Obtenez plus de 50 obligations de combat dans des zones de conflit.|Erhalte mehr als 50 Kampfbelohnungen in Konfliktzonen.|Obtém mais de 50 títulos de combate em zonas de conflito.');
set(300260, 'Consigue más de 15 comprobantes de recompensa por cazar objetivos buscados.|Earn more than 15 bounty vouchers by hunting wanted targets.|Obtenez plus de 15 primes en chassant des cibles recherchées.|Erhalte mehr als 15 Kopfgeldgutscheine durch die Jagd auf gesuchte Ziele.|Obtém mais de 15 vales de recompensa ao caçar alvos procurados.');
set(300270, 'Alcanza reputación Friendly con el Imperio y consigue el permiso de Achenar.|Reach Friendly with the Empire and obtain the Achenar permit.|Atteignez Friendly avec l’Empire et obtenez le permis d’Achenar.|Erreiche Friendly beim Imperium und besorge die Achenar-Genehmigung.|Alcança Friendly com o Império e obtém a licença de Achenar.');
set(300280, 'Consigue la invitación de Colonia Council mediante sus misiones y reputación.|Obtain the Colonia Council invitation through its missions and reputation.|Obtenez l’invitation de Colonia Council grâce à ses missions et à votre réputation.|Erhalte die Einladung von Colonia Council über Missionen und Ruf.|Obtém o convite de Colonia Council através das suas missões e reputação.');
set(400001, 'Completa 10 misiones de restauración o reactivación de asentamientos.|Complete 10 settlement restore or reactivation missions.|Accomplissez 10 missions de restauration ou de réactivation de colonies.|Schließe 10 Wiederherstellungs- oder Reaktivierungsmissionen für Siedlungen ab.|Conclui 10 missões de restauração ou reativação de assentamentos.');
set(400002, 'Recorre al menos 100 AL como pasajero en lanzaderas Apex.|Travel at least 100 ly as a passenger in Apex shuttles.|Parcourez au moins 100 AL comme passager dans les navettes Apex.|Reise mindestens 100 Lj als Passagier in Apex-Shuttles.|Viaja pelo menos 100 AL como passageiro em vaivéns Apex.');
set(400003, 'Completa 10 zonas de conflicto de superficie a pie.|Complete 10 on-foot surface conflict zones.|Terminez 10 zones de conflit de surface à pied.|Schließe 10 Bodenkonfliktzonen zu Fuß ab.|Conclui 10 zonas de conflito de superfície a pé.');
set(400004, 'Vende 5 Opinion Polls a camareros de estaciones.|Sell 5 Opinion Polls to station bartenders.|Vendez 5 Opinion Polls aux barmans des stations.|Verkaufe 5 Opinion Polls an Stations-Barkeeper.|Vende 5 Opinion Polls a empregados de bar das estações.');
set(400005, 'Vende a camareros un total combinado de 15 Multimedia Entertainment, Classic Entertainment o Cat Media.|Sell a combined total of 15 Multimedia Entertainment, Classic Entertainment or Cat Media to bartenders.|Vendez aux barmans un total combiné de 15 Multimedia Entertainment, Classic Entertainment ou Cat Media.|Verkaufe insgesamt 15 Multimedia Entertainment, Classic Entertainment oder Cat Media an Barkeeper.|Vende a empregados de bar um total combinado de 15 Multimedia Entertainment, Classic Entertainment ou Cat Media.');
set(400006, 'Completa 6 misiones encubiertas de tipo Covert Theft o Covert Heist; comprueba las condiciones en el panel del juego.|Complete 6 Covert Theft or Covert Heist missions; check the conditions in the game panel.|Accomplissez 6 missions Covert Theft ou Covert Heist ; vérifiez les conditions dans le panneau du jeu.|Schließe 6 Missionen vom Typ Covert Theft oder Covert Heist ab; prüfe die Bedingungen im Spiel.|Conclui 6 missões Covert Theft ou Covert Heist; confirma as condições no painel do jogo.');
set(400007, 'Reduce tu reputación con Sirius Corporation a Unfriendly o inferior.|Reduce your reputation with Sirius Corporation to Unfriendly or lower.|Réduisez votre réputation avec Sirius Corporation à Unfriendly ou moins.|Senke deinen Ruf bei Sirius Corporation auf Unfriendly oder niedriger.|Reduz a tua reputação com Sirius Corporation para Unfriendly ou inferior.');
set(400008, 'Vende a camareros un total combinado de 20 Biological Sample, Employee Genetic Data o Genetic Research.|Sell a combined total of 20 Biological Sample, Employee Genetic Data or Genetic Research to bartenders.|Vendez aux barmans un total combiné de 20 Biological Sample, Employee Genetic Data ou Genetic Research.|Verkaufe insgesamt 20 Biological Sample, Employee Genetic Data oder Genetic Research an Barkeeper.|Vende a empregados de bar um total combinado de 20 Biological Sample, Employee Genetic Data ou Genetic Research.');
set(400009, 'Vende 5 Smear Campaign Plans a camareros.|Sell 5 Smear Campaign Plans to bartenders.|Vendez 5 Smear Campaign Plans aux barmans.|Verkaufe 5 Smear Campaign Plans an Barkeeper.|Vende 5 Smear Campaign Plans a empregados de bar.');
set(400010, 'Alcanza reputación Friendly con Colonia Council.|Reach Friendly reputation with Colonia Council.|Atteignez une réputation Friendly avec Colonia Council.|Erreiche den Ruf Friendly bei Colonia Council.|Alcança reputação Friendly com Colonia Council.');
set(400011, 'Visita 5 asentamientos distintos en el sistema Colonia.|Visit 5 different settlements in the Colonia system.|Visitez 5 colonies différentes dans le système Colonia.|Besuche 5 verschiedene Siedlungen im System Colonia.|Visita 5 assentamentos diferentes no sistema Colonia.');
set(400012, 'Vende un total combinado de 10 Culinary Recipes o Cocktail Recipes a camareros de estaciones de Colonia.|Sell a combined total of 10 Culinary Recipes or Cocktail Recipes to bartenders at stations in Colonia.|Vendez un total combiné de 10 Culinary Recipes ou Cocktail Recipes aux barmans des stations de Colonia.|Verkaufe insgesamt 10 Culinary Recipes oder Cocktail Recipes an Barkeeper in Colonia-Stationen.|Vende um total combinado de 10 Culinary Recipes ou Cocktail Recipes a empregados de bar em estações de Colonia.');
set(400013, 'Completa las tres referencias: 10 Faction Associates para Baltanos, 10 Digital Designs para Eleanor Bresa y 10 Manufacturing Instructions para Rosa Dayette.|Complete all three referrals: 10 Faction Associates to Baltanos, 10 Digital Designs to Eleanor Bresa and 10 Manufacturing Instructions to Rosa Dayette.|Obtenez les trois recommandations : 10 Faction Associates à Baltanos, 10 Digital Designs à Eleanor Bresa et 10 Manufacturing Instructions à Rosa Dayette.|Erledige alle drei Empfehlungen: 10 Faction Associates an Baltanos, 10 Digital Designs an Eleanor Bresa und 10 Manufacturing Instructions an Rosa Dayette.|Conclui as três referências: 10 Faction Associates para Baltanos, 10 Digital Designs para Eleanor Bresa e 10 Manufacturing Instructions para Rosa Dayette.');

const deliver = (n, item, place) => [
  `Entrega ${n} ${item} en ${place} para desbloquear el acceso.`, `Deliver ${n} ${item} at ${place} to unlock access.`,
  `Livrez ${n} ${item} à ${place} pour débloquer l’accès.`, `Liefere ${n} ${item} bei ${place} ab, um den Zugang freizuschalten.`,
  `Entrega ${n} ${item} em ${place} para desbloquear o acesso.`,
];
const referralText = (n, item, name) => [
  `Tras desbloquear este taller, entrega ${n} ${item} aquí para conseguir la referencia a ${name}. No es una donación para el acceso propio.`,
  `After unlocking this workshop, deliver ${n} ${item} here for the referral to ${name}. This is not a donation for this engineer's own access.`,
  `Après avoir débloqué cet atelier, livrez ici ${n} ${item} pour la recommandation vers ${name}. Ce don ne débloque pas cet ingénieur.`,
  `Liefere nach Freischaltung dieser Werkstatt hier ${n} ${item} für die Empfehlung zu ${name} ab. Diese Spende ist nicht für den eigenen Zugang nötig.`,
  `Após desbloquear esta oficina, entrega aqui ${n} ${item} para obter a referência a ${name}. Não é uma doação para o acesso a este engenheiro.`,
];
function discovery(e, byId) {
  const p = parents[e.id] ?? [];
  if (!p.length) return words('Contacto conocido públicamente; consulta su condición de invitación.|Publicly known contact; check the invitation condition.|Contact connu publiquement ; consultez la condition d’invitation.|Öffentlich bekannter Kontakt; prüfe die Einladungsbedingung.|Contacto de conhecimento público; consulta a condição de convite.');
  if (e.id === 400013) return meeting[e.id];
  const parent = byId.get(p[0]);
  if (e.on_foot) return referralText(...referrals[parent.id]).map((v, i) => [
    `Visita a ${parent.name}. ${v}`, `Visit ${parent.name}. ${v}`, `Rendez visite à ${parent.name}. ${v}`, `Besuche ${parent.name}. ${v}`, `Visita ${parent.name}. ${v}`,
  ][i]);
  return [
    `Trabaja con ${parent.name} hasta rango 3 y al menos un tercio hacia rango 4 para recibir su referencia.`,
    `Work with ${parent.name} to rank 3 and at least one-third towards rank 4 to receive the referral.`,
    `Travaillez avec ${parent.name} jusqu’au rang 3 et au moins un tiers vers le rang 4 pour obtenir la recommandation.`,
    `Arbeite mit ${parent.name} bis Rang 3 und mindestens einem Drittel in Richtung Rang 4 für die Empfehlung.`,
    `Trabalha com ${parent.name} até ao nível 3 e pelo menos um terço até ao nível 4 para obter a referência.`,
  ];
}
function unlock(e) {
  if (donations[e.id]) { const [, , n, item] = donations[e.id]; return deliver(n, item, e.workshop); }
  if ([300150,300210].includes(e.id)) return deliver(10, e.id===300150?'Osmium':'Painite', e.workshop).map((v,i)=>v+' '+words('Debe proceder de tu minería; tenerlo comprado en la bodega no demuestra el requisito.|It must come from your mining; bought cargo does not prove the requirement.|Il doit provenir de votre extraction ; une cargaison achetée ne prouve pas le prérequis.|Es muss aus deinem Bergbau stammen; gekaufte Ladung belegt die Bedingung nicht.|Deve vir da tua mineração; carga comprada não comprova o requisito.')[i]);
  if ([300140,300250,300260,300280].includes(e.id)) {
    const amount = e.id===300140?'1 000 000 / 10 000 000':e.id===300250?'100 000 / 1 000 000':'100 000';
    const item = e.id===300140?words('bonos de combate federales|Federal combat bonds|obligations de combat fédérales|föderale Kampfbelohnungen|títulos de combate federais'):e.id===300250?words('bonos de combate|combat bonds|obligations de combat|Kampfbelohnungen|títulos de combate'):words('comprobantes de recompensa|bounty vouchers|primes|Kopfgeldgutscheine|vales de recompensa');
    return [
      `Entrega ${item[0]} por ${amount} CR en ${e.workshop}. Comprueba la cantidad exigida en el taller; tu saldo no sustituye estos comprobantes.`,
      `Hand in ${item[1]} worth ${amount} CR at ${e.workshop}. Check the amount requested by the workshop; your credit balance cannot replace these vouchers.`,
      `Remettez ${amount} CR en ${item[2]} à ${e.workshop}. Vérifiez le montant demandé à l’atelier ; votre solde ne remplace pas ces justificatifs.`,
      `Gib ${item[3]} im Wert von ${amount} CR bei ${e.workshop} ab. Prüfe den verlangten Betrag in der Werkstatt; dein Guthaben ersetzt diese Belege nicht.`,
      `Entrega ${item[4]} no valor de ${amount} CR em ${e.workshop}. Confirma o valor pedido na oficina; o saldo de créditos não substitui estes comprovativos.`,
    ];
  }
  return meeting[e.id].map((v,i)=>v+' '+[
    `Después visita ${e.workshop}; no hay otra donación documentada para acceder.`, `Then visit ${e.workshop}; no further donation is documented for access.`,
    `Visitez ensuite ${e.workshop} ; aucun autre don d’accès n’est documenté.`, `Besuche danach ${e.workshop}; keine zusätzliche Zugangsspende ist dokumentiert.`,
    `Depois visita ${e.workshop}; não há outra doação documentada para aceder.`,
  ][i]);
}
function reputation(e) {
  if(e.on_foot) return words('Odyssey no usa rangos de reputación G1–G5. Instala modificaciones en las ranuras del equipo.|Odyssey has no G1–G5 engineer reputation ranks. Install modifications in equipment slots.|Odyssey n’utilise pas les rangs de réputation G1–G5. Installez les modifications dans les emplacements d’équipement.|Odyssey hat keine Ingenieur-Rufränge G1–G5. Installiere Modifikationen in Ausrüstungsplätzen.|Odyssey não usa níveis de reputação G1–G5. Instala modificações nas ranhuras do equipamento.').map((v,i)=>referrals[e.id]?`${v} ${referralText(...referrals[e.id])[i]}`:v);
  let result = [
    `Ingenieriza módulos presencialmente en ${e.workshop} para subir reputación y habilitar grados superiores; la ingeniería remota no sube reputación.`,
    `Engineer modules in person at ${e.workshop} to raise reputation and unlock higher grades; remote engineering does not increase reputation.`,
    `Modifiez des modules sur place à ${e.workshop} pour améliorer la réputation et ouvrir les grades supérieurs ; l’ingénierie à distance ne l’améliore pas.`,
    `Modifiziere Module vor Ort bei ${e.workshop}, um Ruf und höhere Grade freizuschalten; Ferningenieurarbeit erhöht den Ruf nicht.`,
    `Modifica módulos presencialmente em ${e.workshop} para aumentar reputação e desbloquear graus superiores; a engenharia remota não aumenta reputação.`,
  ];
  const raw=e.source_text.reputation;
  const extra=raw.includes('Sell commodities')?words('También puedes vender mercancías en este taller.|You can also sell commodities at this workshop.|Vous pouvez aussi vendre des marchandises à cet atelier.|Du kannst in dieser Werkstatt auch Waren verkaufen.|Também podes vender mercadorias nesta oficina.'):raw.includes('Sell exploration')?words('También puedes vender datos de exploración en este taller.|You can also sell exploration data at this workshop.|Vous pouvez aussi vendre des données d’exploration à cet atelier.|Du kannst hier auch Erkundungsdaten verkaufen.|Também podes vender dados de exploração nesta oficina.'):raw.includes('bounty')?words('También puedes entregar los comprobantes de recompensa admitidos por este taller.|You can also hand in the bounty vouchers accepted by this workshop.|Vous pouvez aussi remettre les primes acceptées par cet atelier.|Du kannst hier auch die von der Werkstatt akzeptierten Kopfgeldgutscheine abgeben.|Também podes entregar os vales de recompensa aceites nesta oficina.'):null;
  if(extra)result=result.map((v,i)=>`${v} ${extra[i]}`);
  return result;
}

export function guideContent(entries) {
  const byId=new Map(entries.map(e=>[e.id,e]));
  return entries.map(e=>{
    if(!meeting[e.id]) throw new Error(`No guide for engineer ${e.id}`);
    const slug=e.unlock.requirement_key.slice('unlock-'.length);
    const d=discovery(e,byId), r=reputation(e), u=unlock(e);
    const prerequisites=(parents[e.id]??[]).map(id=>({engineer_id:id,description_key:`unlock-prerequisite-${slug}-${id}`}));
    const content={};
    for(let i=0;i<langs.length;i++) {
      const lines={ [`unlock-discovery-${slug}`]:d[i], [e.unlock.requirement_key]:meeting[e.id][i], [e.unlock.invite_key]:u[i], [`unlock-reputation-${slug}`]:r[i] };
      for(const p of prerequisites) {
        if(e.id!==400013) { lines[p.description_key]=d[i]; continue; }
        const parent=byId.get(p.engineer_id).name;
        const [n,item]=referrals[p.engineer_id];
        lines[p.description_key]=[
          `Desbloquea a ${parent} y entrégale ${n} ${item} para que te presente a Yi Shen.`,
          `Unlock ${parent} and give them ${n} ${item} for the referral to Yi Shen.`,
          `Débloquez ${parent} et remettez-lui ${n} ${item} pour obtenir la recommandation vers Yi Shen.`,
          `Schalte ${parent} frei und übergib dort ${n} ${item} für die Empfehlung zu Yi Shen.`,
          `Desbloqueia ${parent} e entrega-lhe ${n} ${item} para obter a referência a Yi Shen.`,
        ][i];
      }
      content[langs[i]]=lines;
    }
    const contribution=donations[e.id];
    return {id:e.id,unlock:{...e.unlock,contribution:contribution?{item:contribution[0],kind:contribution[1],amount:contribution[2]}:null,
      discovery_key:`unlock-discovery-${slug}`,reputation_key:`unlock-reputation-${slug}`,prerequisites,
      guide_sources:[source, ...(e.on_foot?[update]:[]), ...(e.id===300010?['https://inara.cz/elite/engineer/19/']:[])],guide_verified:true},content};
  });
}
export function enrichEngineerGuides(engineers) {
  const rows=guideContent(engineers.entries);
  return {...engineers,entries:engineers.entries.map((e,i)=>({...e,unlock:rows[i].unlock}))};
}
if(process.argv[1]===fileURLToPath(import.meta.url)) {
  const path='crates/tmc-modules/data/engineers.json';
  const engineers=JSON.parse(readFileSync(path,'utf8'));
  const rows=guideContent(engineers.entries);
  writeFileSync(path,JSON.stringify(enrichEngineerGuides(engineers),null,2)+'\n');
  for(const lang of langs) {
    const path=`locales/${lang}/engineering-data.ftl`;
    const text=readFileSync(path,'utf8').replace(/\r\n/g,'\n').replace(/^unlock-[^\n]*(?:\n[ \t]+[^\n]*)*\n?/gm,'').replace(/^# F16: guía de acceso y progresión;[^\n]*\n?/gm,'').trimEnd();
    writeFileSync(path,text+'\n\n# F16: guía de acceso y progresión; redacción TMC, ver docs/research/2026-11-05-guia-ingenieros.md\n'+rows.flatMap(e=>Object.entries(e.content[lang]).map(([k,v])=>`${k} = ${v}`)).join('\n')+'\n');
  }
}
