# Porg

CLI tool for organizing ePrint papers

## Quick install

``` shell
cargo install --git https://github.com/jannikspiessens/porg.git
```

## Usage

``` shell
porg --help
```

### qutebrowser

Porg can also be called through a [qutebrower](https://github.com/qutebrowser/qutebrowser) userscript. Add the following line to your qutebrowser config file.

``` Python
config.bind('<Ctrl-o>', 'hint links userscript ~/.cargo/bin/porg')
```

