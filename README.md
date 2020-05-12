# jorup: the jormungandr's installer and manager

[![Continuous integration][gh-actions-badge]][gh-actions-link]

## Installation

See https://input-output-hk.github.io/jorup

## Usage

`jorup` is a command line tool to help manage the node under different testnets
and parameters.

### Downloading/updating blockchain configurations

`jorup` is designed to provide multiple network configurations. Some
configurations (including the incentivized testnet) are maintained in the
`jorup` repository. To download or update them run:

	jorup blockchain update

### Installing/updating the node

In addition to managing multiple blockchain configurations, you can install,
update and have several different versions of `jormungandr`.

You can download the latest version compatible with a particular network. For
example, to install the latest `jormungandr` run:

	jorup node install

The same command should be used to update the node.

You can also download a version compatible with a particular network:

	jorup node install itn

or just download the version you want:

	jorup node install -v 0.8.17

To install today's nightly version (**do it on your own risk**):

	jorup node install nightly

### Starting the node

The node can be started with `jorup run`. You should provide the name of the
network you want to connect:

	jorup run itn

If you want to run a specific version of jormungandr, please specify a version:

	jorup run itn -v 0.8.17

To run the node in the background, use the `--daemon` flag.

### Getting the node's info

	jorup info itn

Get the info of a background running node. You should specify the network name.

### Shuting down a background node

	jorup shutdown itn

Shutdown a background running node. You should specify the network name.

### Customizing the node configuration

The first way to customize a node configuration is to provide additional flags
supported by `jormungandr`:

	jorup run itn -- --enable-explorer

This way of configuring `jormungandr` is limited, because `jorup` also uses
command line arguments to configure `jormungandr`. For better configuration you
may want to use a custom configuration file. Steps to build a custom
configuration file are:

1. Export the current configuration:

   ```jorup defaults itn > config.yaml```

2. Edit a new config in any way you want.

3. Get the genesis block hash for your network (you will need to provide it with
   a flag):

   ```jorup blockchain list```

4. Run `jorup` with your configuration:

   ```jorup run itn --config config.yaml -- --genesis-block-hash 8e4d2a343f3dcf9330ad9035b3e8d168e6728904262f2c434a4f8f934ec7b676```

Using `--config` prevents `jorup` from adding any additional configuration flags
when starting `jormungandr`, so you get more freedom with the command line
options.

## License

Copyright Input Output HK Ltd and contributors.

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

[gh-actions-link]: https://github.com/input-output-hk/jorup/actions?query=workflow%3A%22Continuous+integration%22
[gh-actions-badge]: https://github.com/input-output-hk/jorup/workflows/Continuous%20integration/badge.svg
