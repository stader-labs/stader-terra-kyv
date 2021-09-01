import { MsgStoreCode, getCodeId, isTxError } from "@terra-money/terra.js";
import { client, wallet } from "./clientAndWallet";

const fs = require("fs");

async function uploadContract(): Promise<string> {
  const filepath = __dirname + "/../artifacts/stader_terra_kyv.wasm";
  const storeCode = new MsgStoreCode(
    wallet.key.accAddress,
    fs.readFileSync(filepath).toString("base64")
  );
  console.log(wallet.key.accAddress);

  const storeCodeTx = await wallet.createAndSignTx({
    msgs: [storeCode],
  });
  const storeCodeTxResult = await client.tx.broadcast(storeCodeTx);

  console.log(storeCodeTxResult);

  if (isTxError(storeCodeTxResult)) {
    throw new Error(
      // "bar"
      // `store code failed. code: ${storeCodeTxResult.code}, codespace: ${storeCodeTxResult.codespace}, raw_log: ${storeCodeTxResult.raw_log}`
      `store code failed. code: ${storeCodeTxResult.code}, codespace: ${storeCodeTxResult.codespace}`
    );
  }

  const codeId = getCodeId(storeCodeTxResult);
  console.log(`Code id is: ${codeId}`);
  return codeId;
}

export default uploadContract;
