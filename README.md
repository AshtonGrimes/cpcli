# cpcli

`cpcli` (crypto price cli) uses the CoinGecko API to display current exchange rates for cryptocurrencies. 
Being that this program is among my first publically available software projects, I welcome any recommendations on how to improve it.

## Usage

Options can be used together in any order.

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

     146.10  -8.610%

$ cpcli -c eur cardano chainlink

     Cardano  0.51017 -7.992%
     Chainli- 12.70   -8.520%

$ cpcli tether:40

   40 USD -> 39.920 Tether

$ cpcli -t 5 

   1   Bitcoin  87.59K  -0.527%
   2   Ethereum 3.23K   -2.978%
   3   Tether   1.00    -0.040%
   4   Solana   205.52  -6.647%
   5   BNB      612.36  -6.273%
```

## License
`cpcli` is free software, licensed under the GNU General Public license version 3.0 (GPL-3.0). See the LICENSE file for details.
