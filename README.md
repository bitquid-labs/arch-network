# Arch Network

To build the program, run:

```bash
git clone git@github.com:bitquid-labs/arch-network.git
cd app/program/src
cargo-build-sbf
```

Before you deploy, you first have to start the validator locally:

```bash
arch-cli validator start
```

To deploy and store the program on the Arch Network, run:

```bash
arch-cli deploy
```

To stop the validator, simply issue the corresponding stop command:

```bash
arch-cli validator stop
```

## Handling Cross Chain Communications on Arch

Arch proposes a bridge-free solution for Bitcoin by leveraging its UTXO (Unspent Transaction Output) model. It enables different Bitcoin-based L2s and meta-protocols to communicate directly, without needing to bridge assets back and forth to the base Bitcoin layer.

#### Cross Program Invocation (CPI)

Cross-Program Invocation is a key feature that enables smart contracts on different protocols to interact with each other seamlessly within a shared ecosystem.
This address the need for contract calls among our different contract without the need of Hyperlane, LayerZero and bridges as we were exploring.

#### [Bridgeless Execution](https://arch-network.gitbook.io/arch-documentation/fundamentals/introducing-arch/bridgeless-execution)

The mechanism would eradicate the risks, costs and low latency that comes with token and state transfer across bridges, by executing tx directly on the Bitcoin base layer without the tokens needing to leave their original chain.

This would allow:

- Cover contracts on various Bitcoin L2s, when a user purchases a cover on another network, the state transition of the transaction (the payment) can be anchored on the Bitcoin base layer without moving tokens across chains via bridges.

- (Possibly) allow for pool deposits from various networks, by anchoring the transaction on Bitcoin and updating the state on the pool contract.

- (Possibly) since transactions are executed/ anchored directly on the Bitcoin base layer, BQToken usage can happen on any L2 or partner chain, with the results recorded back to the base layer. This might bring the possibilties of using the BQToken across various networks for cover purchases as well.

### Program ID

Pool : 6a4e49d74bce4744005444d46f4d52ac430b236a8abd609465a9262340e36235

BqBTC: 2c031a18df0c598360d09ab8114fcd0aa1186cf67e8d9b603bfac263da7ccd6b
