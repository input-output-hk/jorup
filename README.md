# jorup: the jormungandr's installer and manager

| CI           | Build Status                                             |
|--------------|----------------------------------------------------------|
| Travis       | [![Travis Build Status][travis-badge]][travis-url]       |
| AppVeyor     | [![AppVeyor Build Status][appveyor-badge]][appveyor-url] |

## Installation

Under construction

## Usage

`jorup` is a command line tool to help manage the node under different testnets
and parameters. While it is still very much **WORK IN PROGRESS** it can already
be used to follow up with the appropriate installation for a given `Channel`.

### `Channel`

a `Channel` is a possible testnet parameter. There are currently only 2 different
kind of `channels`: `stable` or `nightly`.

* `nightly` are likely to break or to be deprecated so it is not recommended to
  host them for long or to have long plans on them. Instead they are meant for
  testing early features, specific blockchain parameters or a given version of
  the node.
* `stable` are... _not yet defined_.

### Updating the local installation

The following command will update the locally installed default channel.

```jorup update```

You can update a specific channel by specifying it on the command line 
options:

* `jorup update nightly`: will update to the latest version of nightly available
  (i.e. it may change to a new version of the blockchain), and will update to the
  latest release available compatible with this `channel`.
* `jorup update '0.5.5-nightly (2019-10-04)' will not update to a new default channel
  but will instead only update that specific version of a the `nightly` channel.

To make a default `channel` the default, simply add the command line parameter `--default`.

### Starting the node

```jorup run```

will start the default node. Specify the `channel` you want to start if you want to start
another channel than the default.

If you want to start the node in the background, simply add `--daemon`.


### Getting the node's info

```jorup info```

Get the info of a background running node. Specify the `channel` you want if you want to get
info for a specific `channel`.

### Shuting down a background node

```jorup shutdown```

Shutdown a background running node. Specify the `channel` you want if you want to shutdown
a specific `channel`.

## License

Copyright Input Output HK Ltd and contributors.

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

<!-- Badges -->
[travis-url]: https://travis-ci.com/input-output-hk/jorup
[travis-badge]: https://travis-ci.com/input-output-hk/jorup.svg
[appveyor-url]: https://ci.appveyor.com/project/NicolasDP/jorup
[appveyor-badge]: https://ci.appveyor.com/api/projects/status/github/input-output-hk/jorup?svg=true