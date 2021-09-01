import kyvContract from "./contact-api";

const validatorAddr = "terravaloper1dcegyrekltswvyy0xy69ydgxn9x8x32zdy3ua5";

// Cron timestamps
const timestamps1 = [
  1630407446, 1630408145, 1630409423, 1630410965, 1630413801, 1630415609,
  1630420656,
];

const timestamps2 = [1630508284, 1630508391];

const run = async () => {
  /*
    #Query Get the current state/config of the contract
    await kyvContract.query.getCurrentState().then(console.log, console.log);

    #Query Compute Apr a validator for the given time interval
    await kyvContract.query
    .computeValidatorAPR(1630508284, 1630508391, validatorAddr)
    .then(console.log, console.log);

    #Query Comput Apr of all validators for the given time interval
    await kyvContract.query
    .computeAllValidatorsAPRs(1630508284, 1630508391)
    .then(console.log, console.log);

    #Query - Get [rewards and delegated amount] at a given time
    const res = await kyvContract.query.getHistoryByTime(1630508391);
    console.log(JSON.stringify(res));

    #Execute - Add validator && Run cron to compute metrics
     await kyvContract.execute.addNewValidator(validatorAddr); //.then(console.log, console.log);
     await kyvContract.execute.recordMetrics();
  */
};

run().then(console.log, console.error);
