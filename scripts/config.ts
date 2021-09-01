// export const contractAddress = "terra1fr4rzvq4xrdy080cqcvxhzmtph0pzavcejejh7"; // KYV Version 0.0.1
export const contractAddress = "terra1wa3t5m8c05s2ndu22xp2s39jkrm3h0k7h2stq3"; // KYV Version 0.0.2

// TODO: chappie: mnemonic_keys for prod will be from env vars
// the wallets/mnemonic_keys in prod must not hold more than 10 luna at any given point of time
const configs = {
  development: {
    environment: "development",
    mnemonic_keys: [
      // "mosquito country behave theme lunar coast duty maximum office net nest arena possible figure month fat rent pepper slow exhaust sunny humble media jazz",
      "quality vacuum heart guard buzz spike sight swarm shove special gym robust assume sudden deposit grid alcohol choice devote leader tilt noodle tide penalty",
    ],
    finderUrlForTxs: "https://finder.terra.money/bombay-0008/tx/",
    fcdUrl: "http://terra-dev.staderlabs.com:3060",
    chainId: "localterra",
    initialValidators: ["terravaloper1dcegyrekltswvyy0xy69ydgxn9x8x32zdy3ua5"],
    // {
    //   "name": "staderlabs-terra-net",
    //   "chainID": "localterra",
    //   "lcd": "https://fcd.staderlabs.com:1317/",
    //   "fcd": "https://fcd.staderlabs.com:3060/",
    //   "localterra": false
    // }
  },
  test: {
    environment: "test",
    mnemonic_keys: [
      "mosquito country behave theme lunar coast duty maximum office net nest arena possible figure month fat rent pepper slow exhaust sunny humble media jazz",
      "obvious chat where slide demand sudden chunk general custom twin limb surprise frown unlock praise grow fade rule inflict adapt urge spoon inch enlist",
      "sudden option cradle donor same stone joy ancient choice glimpse rigid winter double announce fresh industry bonus off casino remove olympic relief bar save",
      "hedgehog gauge identify gaze total town warfare tragic ten spot stick vessel purpose olympic increase glance whisper slim soup online sugar celery acid angry",
      "blood feature enrich meat vanish lock empty vacant emerge crater bounce fat rent digital eye injury border decide clown client rude result ensure base",
      "eager rabbit symptom reason enrich garbage common explain choose panther miracle card thank knock first sand suggest alter regular ridge sound wonder panther mother",
      "meat differ system tackle scan top kind video anger inch usual dune ride flat spoil local shop ribbon canoe survey found pencil boss invest",
      "notice today hammer put dynamic problem heart make year casual various gas penalty typical grunt castle clock raccoon legal true letter flash traffic motion",
      "walnut gaze across elder love slide struggle genuine tongue share horror palace fee embrace sample curtain hedgehog void oil lobster prevent guide plug barrel",
      "circle regret estate layer hire pumpkin again assault decrease labor cabin badge honey cruel ability trip sadness agent fan foam relief boring other cannon",
      "cliff material gun avoid lunar wasp shy divide image ocean impact law actress payment orchard crash day horror gap merry robot shoulder crane tuna",
      "fire budget logic describe lesson surge fashion add index borrow medal hub you leopard negative toss limit fence horn wife sight shed unhappy amazing",
      "regular world pyramid meadow craft merge cream typical indicate promote vessel employ dirt lucky auction concert unveil keen recipe attract swap idea unhappy print",
      "lyrics pond market two idea once point mammal escape blush trophy mom raven estate armed sphere tribe early rapid wheat steel rhythm armed oyster",
      "gesture silver loan property infant weapon crunch upset song fragile powder armed public token drip virus concert rival brother often ramp ankle attract scissors",
      "decorate notice ridge picnic swear fitness film wear grab youth hurdle recall canvas insect dad jaguar cherry skin embody foster pepper logic hunt review",
      "manual online purity speed cigar host tell again ostrich thrive convince barrel jaguar equal fade oppose harvest estate ring dog hover scheme cat few",
      "fire settle detail thing topple daring gesture install double emotion smart aunt market kitchen wonder crazy goddess forest wash crash draw umbrella vault dutch",
      "sense write vote cloud nice tongue dizzy duck indoor symbol lion champion method laptop path menu tourist begin wife artefact pride test zoo despair",
      "juice general audit wave change live snake typical sketch excuse tooth opera snap trade benefit entire nut oak catch repair olympic health maze hand",
      "fox section rude snow question crumble metal curve buddy believe announce calm december lounge make flower novel stage flower faculty action very amateur cause",
      "rug fee strong pumpkin fence glove wrap minimum shoot naive early rule siren game brisk approve team crucial obey course raw chimney thumb spin",
      "same solid lyrics cargo link industry pattern episode soft lock smoke sad invite local much flight pipe sadness cross edit cheap flush seat craft",
      "catch awesome angry affair install wheat detail juice place hero image tissue pull general note sphere art cat rather guard hamster outside dice primary",
      "weapon stool fox razor boil unusual kitchen pool flat endless sound olive enter toe cinnamon elder manual coin skate dress domain core regret crawl",
    ],
    finderUrlForTxs: "https://finder.terra.money/bombay-0008/tx/",
    fcdUrl: "http://terra-dev.staderlabs.com:3060",
    chainId: "localterra",
    initialValidators: [
      "terravaloper1pfkp8qqha94vcahh5pll3hd8ujxu8j30xvlqmh",
      "terravaloper1peytphgvnmaz4fah8daww2yaugpw27cdkvcywa",
    ],
  },
  production: {
    environment: "production",
    finderUrlForTxs: "https://finder.terra.money/bombay-0008/tx/",
    fcdUrl: "",
    chainId: "",
  },
};
// TODO: Environment Should be dynamic
export const config = configs.development;

// stick ketchup gossip decline horn situate protect subject achieve flock palace escape still more pelican snake gate olive creek define monster venue empty chaos
