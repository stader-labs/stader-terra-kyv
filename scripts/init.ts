import {
  getContractAddress,
  MsgInstantiateContract,
  isTxError,
} from "@terra-money/terra.js";
import { client, wallet } from "./clientAndWallet";

async function initiateContract(codeId: number): Promise<string> {
  const instantiate = new MsgInstantiateContract(
    wallet.key.accAddress,
    wallet.key.accAddress,
    +codeId, // code ID
    {
      vault_denom: "uluna",
      amount_to_stake_per_validator: "10000000",
      batch_size: 10,
    }, // InitMsg
    { uluna: 10000000 } // init coins
  );

  const instantiateTx = await wallet.createAndSignTx({
    msgs: [instantiate],
  });
  const instantiateTxResult = await client.tx.broadcast(instantiateTx);

  console.log(instantiateTxResult);

  if (isTxError(instantiateTxResult)) {
    throw new Error(
      // "foo"
      // `instantiate failed. code: ${instantiateTxResult.code}, codespace: ${instantiateTxResult.codespace}, raw_log: ${instantiateTxResult.raw_log}`
      `instantiate failed. code: ${instantiateTxResult.code}, codespace: ${instantiateTxResult.codespace}`
    );
  }

  const contractAddress = getContractAddress(instantiateTxResult);

  console.log(`Contract address is ${contractAddress}`);

  return contractAddress;
}

export default initiateContract;
