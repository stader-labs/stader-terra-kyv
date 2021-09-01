import { MsgExecuteContract } from "@terra-money/terra.js";
import { client, wallet } from "./clientAndWallet";
import { contractAddress } from "./config";

function log<T>(args: T): T {
  console.log(JSON.stringify(args, null, 2));
  return args;
}

const walletAddress = wallet.key.accAddress;
const queryApi = async (query: any) => {
  return client.wasm.contractQuery(contractAddress, query);
};

const execApi = async (execMsgs: any[], amount: number) => {
  const executeTx = await wallet.createAndSignTx({
    msgs: execMsgs.map(
      (msg) =>
        new MsgExecuteContract(
          walletAddress, // sender
          contractAddress, // contract address
          msg, // handle msgs
          amount === 0 ? undefined : { uluna: amount } // Coins
        )
    ),
  });

  return await client.tx.broadcast(executeTx);
};

const updateRecordsToUpdatePerRun = async (no: number) => {
  return await execApi([{ update_records_to_update_per_run: { no } }], 0);
};

const addNewValidator = async (addr: string) => {
  return await execApi([{ add_validator: { addr } }], 10000000);
};

const recordMetrics = async () => {
  return await execApi(
    [{ record_metrics: { timestamp: Math.floor(Date.now() / 1000) } }],
    0
  );
};

const getHistoryByTime = async (timestamp: number) => {
  return (await queryApi({
    get_history_by_time: { timestamp },
  })) as ValidatorMetric[];
};

const getState = async () => {
  return await queryApi({ get_current_state: {} });
};

async function computeAllValidatorsAPRs(
  timestamp1: number,
  timestamp2: number
) {
  return await queryApi({
    get_all_aprs_by_interal: { timestamp1, timestamp2 },
  });
}

async function computeValidatorAPR(
  timestamp1: number,
  timestamp2: number,
  addr: string
) {
  return await queryApi({
    get_apr_by_validator: { timestamp1, timestamp2, addr },
  });
}

const kyvContractApi = {
  query: {
    getState,
    getHistoryByTime,
    computeAllValidatorsAPRs,
    computeValidatorAPR,
  },
  execute: {
    addNewValidator,
    recordMetrics,
    updateRecordsToUpdatePerRun,
  },
};

export default kyvContractApi;

// function computeAPR(h1: ValidatorMetric, h2: ValidatorMetric) {
//   const numerator = (+h2.rewards - +h1.rewards) * (365 * 86400) * 100;
//   const denominator = +h2.delegated_amount * (h2.timestamp - h1.timestamp);
//   return (numerator / denominator).toFixed(3) + "%";
// }

type ValidatorMetric = {
  addr: string;
  rewards: string;
  delegated_amount: string;
  timestamp: number;
};
