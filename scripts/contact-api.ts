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
  await execApi([{ update_records_to_update_per_run: { no } }], 0);
};

const addNewValidator = async (addr: string) => {
  await execApi([{ add_validator: { addr } }], 10000000);
};

const recordMetrics = async () => {
  await execApi(
    [{ record_metrics: { timestamp: Math.floor(Date.now() / 1000) } }],
    0
  );
};

const getHistoryByTime = async (timestamp: number) => {
  return (await queryApi({
    get_history_by_time: { timestamp },
  })) as ValidatorMetric[];
};

const getCurrentState = async () => {
  await queryApi({ get_current_state: {} });
};

type ValidatorMetric = {
  addr: string;
  rewards: string;
  delegated_amount: string;
  timestamp: number;
};

function computeAPR(h1: ValidatorMetric, h2: ValidatorMetric) {
  const numerator = (+h2.rewards - +h1.rewards) * (365 * 86400) * 100;
  const denominator = +h2.delegated_amount * (h2.timestamp - h1.timestamp);
  return (numerator / denominator).toFixed(3) + "%";
}

async function computeAPRByInterval(timestamp1: number, timestamp2: number) {
  const res1 = await getHistoryByTime(timestamp1);
  const result1 = { ...res1[0], timestamp: timestamp1 };

  const res2 = await getHistoryByTime(timestamp2);
  const result2 = { ...res2[0], timestamp: timestamp2 };

  const apr = computeAPR(result1, result2);

  console.log(`APR for the given interval is = ${apr}`);

  return apr;
}

async function computeAPRByIntervals(...timestamps: number[]) {
  const results = [];
  for (const time of timestamps) {
    const res = await getHistoryByTime(time);
    const currentResult = { ...res[0], timestamp: time };
    let apr = "-";
    if (results.length > 0) {
      const lastResult = results[results.length - 1];
      apr = computeAPR(lastResult, currentResult);
    }
    results.push({ ...currentResult, apr });
  }

  console.log(
    `Apr's for the given intervals are = ${results
      .map((r) => r.apr)
      .slice(1)
      .join(" | ")}`
  );

  return results.map((r) => r.apr);
}

const kyvContractApi = {
  query: {
    getCurrentState,
    getHistoryByTime,
    computeAPRByIntervals,
    computeAPRByInterval,
  },
  execute: {
    addNewValidator,
    recordMetrics,
    updateRecordsToUpdatePerRun,
  },
};

export default kyvContractApi;
