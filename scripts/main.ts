import kyvContract from "./contact-api";

const validatorAddr = "terravaloper1dcegyrekltswvyy0xy69ydgxn9x8x32zdy3ua5";

// Cron timestamps
const timestamps = [
  1630407446, 1630408145, 1630409423, 1630410965, 1630413801, 1630415609,
  1630420656,
];

const run = async () => {
  /*
    query-Get the current state/config of the contract

    await kyvContractApi.query.getCurrentState() //.then(console.log, console.log);
  */
  //-
  /*
    query-Get [rewards and delegated amount] at a given time

    const { rewards, delegated_amount } =
    await kyvContractApi.query.getHistoryByTime(1630420656)[0];
    console.log(`Rewards = ${rewards} Delegated Amount = ${delegated_amount}`);
  */
  //-
  // query-Compute APR by interval(s)

  await kyvContract.query.computeAPRByInterval(1630407446, 1630420656);
  await kyvContract.query.computeAPRByIntervals(
    1630407446,
    1630408145,
    1630409423,
    1630410965,
    1630413801
  );
  //
  /*
    execute- Add validator | Run cron to compute metrics
    await kyvContractApi.execute.addNewValidator(validatorAddr) //.then(console.log, console.log);
    await kyvContractApi.execute.recordMetrics()
    */
};

run().then(console.log, console.error);
