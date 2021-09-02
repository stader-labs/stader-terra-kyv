# Stader Terra KYV - _Know your validator!_

Stader Terra KYV is a Terra smart contract for tracking the validator metrics regularly.

## Metrics we track

- APR
- Commision
- Slashing
- Self delegation
- Voting power
- Others.

## Stack

| Technology | Usage                                                         |
| ---------- | ------------------------------------------------------------- |
| [TerraJs]  | Interact with the Terra blockchain / Smart contract           |
| [Rust]     | Contract Language                                             |
| [CosmWasm] | New smart contracting platform built for the cosmos ecosystem |

## Installation

- Stader Terra KYV requires [Node.js](https://nodejs.org/) to run [TerraJs] functions.
- [Rust] and [Docker] to run and build the contract.
- [More info][cosmwasm-installation] about the setup & installation.

#### Contract

```sh
cd stader-terra-kyv
cargo build # Compile local packages and all of their dependencies.
cargo test # Run unit tests
./compile # Build the contract (You will see "./artifacts/stader_terra_kyv.wasm" file)
```

#### Scripts

```sh
cd stader-terra-kyv/scripts
npm install # Install all dependencies.
npx ts-node deploy.ts # Upload & Initialize the contract (Need a config, client, wallet info)
npx ts-node main.ts # Has scripts to interact with the contract
```

#### Interacting scripts

> Query - Get the current state/config of the contract

```js
kyvContract.query.getState();
```

> Query - Compute Apr a validator for a given time interval

```js
kyvContract.query.computeValidatorAPR(t1, t2, validatorAddr);
```

> Query - Compute Apr of all validators for a given time interval

```js
kyvContract.query.computeAllValidatorsAPRs(t1, t2);
```

> Query - Get validator metrics at a given time

```js
kyvContract.query.getValidatorMetricsByTime(t);
```

> Query - Get All the validators metrics at a given time

```js
kyvContract.query.getAllValidatorMetricsByTime(t);
```

> Execute - Add validator

```js
kyvContract.execute.addNewValidator(validatorAddr);
```

> Execute - Run cron to record validator metrics

```js
kyvContract.execute.recordMetrics();
```

#### CI Support

We have template configurations for both GitHub Actions and Circle CI in the generated project, so you can get up and running with CI right away.
One note is that the CI runs all `cargo` commands with `--locked` to ensure it uses the exact same versions as you have locally. This also means you must have an up-to-date `Cargo.lock` file, which is not auto-generated.

The first time you set up the project (or after adding any dep), you should ensure the `Cargo.lock` file is updated, so the CI will test properly. This can be done simply by running `cargo check` or `cargo unit-test`.

## License

MIT

**Free Software, Hell Yeah!**

[//]: # "These are reference links used in the body of this note and get stripped out when the markdown processor does its job. There is no need to format nicely because it shouldn't be seen."
[terrajs]: https://terra-money.github.io/terra.js/
[rust]: https://www.rust-lang.org/
[cosmwasm]: https://docs.cosmwasm.com/docs/0.16/
[stader-terra-kyv]: https://github.com/stader-labs/stader-terra-kyv
[git-repo-url]: https://github.com/stader-labs/stader-terra-kyv
[docker]: https://www.docker.com/get-started
[cosmwasm-installation]: https://docs.cosmwasm.com/docs/0.16/getting-started/installation
