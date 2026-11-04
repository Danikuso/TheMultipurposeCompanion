
export interface UiModuleDef {
  id: string;
  icon: string;
  titleKey: string;
  cards: string[];
}

export const modules: UiModuleDef[] = [
  {
    id: 'engineering',
    icon: 'wrench',
    titleKey: 'nav-engineering',
    cards: ['materials', 'engineers', 'eng-ship', 'eng-flow', 'shopping', 'trades', 'sources', 'eng-alerts'],
  },
  {
    id: 'dashboard',
    icon: 'gauge',
    titleKey: 'nav-dashboard',
    cards: [
      'dashboard-cmdr', 'ship-detail', 'dashboard-location', 'dashboard-status',
      'dashboard-route', 'dashboard-session', 'dashboard-alerts',
    ],
  },
  {
    id: 'ship',
    icon: 'rocket',
    titleKey: 'nav-ship',
    cards: ['ship-detail', 'ship-outfitting-search'],
  },
  {
    id: 'navigation',
    icon: 'route',
    titleKey: 'nav-navigation',
    cards: ['route', 'jump', 'planner'],
  },
  {
    id: 'exploration',
    icon: 'orbit',
    titleKey: 'nav-exploration',
    cards: ['exploration-system-bodies'],
  },
  {
    id: 'exobiology',
    icon: 'leaf',
    titleKey: 'nav-exobiology',
    cards: ['exobiology-exobiology'],
  },
  {
    id: 'combat',
    icon: 'crosshair',
    titleKey: 'nav-combat',
    cards: ['target', 'ship', 'hunting', 'cz', 'ax'],
  },
  {
    id: 'mining',
    icon: 'pickaxe',
    titleKey: 'nav-mining',
    cards: ['mining-targets', 'prospector', 'mining-session', 'sell'],
  },
  {
    id: 'trade',
    icon: 'coins',
    titleKey: 'nav-trade',
    cards: ['market', 'cargo', 'trade-route', 'trade-session', 'search'],
  },
  {
    id: 'missions',
    icon: 'clipboard-list',
    titleKey: 'nav-missions',
    cards: ['missions', 'massacre', 'mission-alerts', 'passengers', 'tourism', 'rewards', 'community-goals'],
  },
  {
    id: 'carrier',
    icon: 'anchor',
    titleKey: 'nav-carrier',
    cards: ['carrier-overview', 'carrier-jump', 'carrier-finances'],
  },
  {
    id: 'colonisation',
    icon: 'hammer',
    titleKey: 'nav-colonisation',
    cards: ['colonisation-progress', 'colonisation-supply'],
  },
  {
    id: 'powerplay',
    icon: 'crown',
    titleKey: 'nav-powerplay',
    cards: ['powerplay-status'],
  },
  {
    id: 'bgs',
    icon: 'network',
    titleKey: 'nav-bgs',
    cards: ['bgs-system'],
  },
  {
    id: 'accounts',
    icon: 'globe',
    titleKey: 'nav-accounts',
    cards: ['accounts-accounts'],
  },
  {
    id: 'crime',
    icon: 'scale',
    titleKey: 'nav-crime',
    cards: ['crime-crime'],
  },
];
