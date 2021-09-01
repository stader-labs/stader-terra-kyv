import { LCDClient, MnemonicKey } from "@terra-money/terra.js";
import { config } from "./config";

// connect to a network
export const client = new LCDClient({
  URL: config.fcdUrl,
  chainID: config.chainId,
  // URL: "https://bombay-fcd.terra.dev/",
  // chainID: 'bombay-0008',
  // gasPrices: { uluna: 0.5 },
  // gasAdjustment: 1.4
});

const mk = new MnemonicKey({
  // this is main account
  // mnemonic: "talent hurry ignore beach common syrup wool vintage midnight awake aspect patient nuclear face mesh black trust upon fine fan coconut adapt cereal crucial",
  // temp localterra account
  mnemonic:
    "stick ketchup gossip decline horn situate protect subject achieve flock palace escape still more pelican snake gate olive creek define monster venue empty chaos",
});

export const wallet = client.wallet(mk);
// export const managerAddress = wallet.accountNumber();

// stick ketchup gossip decline horn situate protect subject achieve flock palace escape still more pelican snake gate olive creek define monster venue empty chaos

// notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius
