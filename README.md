# cpcli

`cpcli` (crypto price cli) uses the CoinGecko API to display current exchange rates for cryptocurrencies. 
Being that this program is among my first publically available software projects, I welcome any recommendations on how to improve it.

## Usage

The source code is the configuration; to modify the padding and column widths, change any of the global variables under the global variable comment. `grep -n global $(which cpcli)`.

Options can be used in any order.

```
$ cpcli -h
            
Options:
    [TOKEN]              Current value of TOKEN in fiat
    [TOKEN:N]            Convert N of fiat to TOKEN
    [N:TOKEN]            Convert N of TOKEN to fiat
    -c C | --currency C  Use fiat currency C (default is "usd")
    -h | --help          Display this help message
    -t N | --top N       Top N tokens by market cap   

$ cpcli monero

     Monero   199.03  +4.706%

$ cpcli -c eur cardano chainlink

     Chainli- 19.65   +2.254%
     Cardano  0.90    +1.655%

$ cpcli tether:40

   40 USD -> 40.01 Tether

$ cpcli -t 5

   1   Bitcoin  94.36K  +1.219%
   2   Ethereum 3.24K   -0.095%
   3   Tether   0.99984 +0.004%
   4   XRP      2.35    +2.574%
   5   BNB      690.93  +0.375%

```

## License
`cpcli` is free software, licensed under the GNU General Public license version 3.0 (GPL-3.0). See the LICENSE file for details.
