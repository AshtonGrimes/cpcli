use std::cmp;
use std::{env, process};
use reqwest::blocking::Client;

struct Flags {
    c: String,
    t: u16,
}

struct EvalToken {
    name: String,
    amount: f64,
    fiat_to_token: bool
}
/* 
    Reading JSON objects to a predefined struct only works for the top 
    tokens list; to use a struct for the basic API endpoint (used for 
    specific tokens), the names of the struct fields would need to include 
    the currency, which is only known at runtime
*/ 
#[derive(serde::Deserialize)]
struct TopToken {
    market_cap_rank: u8,
    name: String,
    current_price: f64,
    price_change_percentage_24h: f64
}

macro_rules! fail {
    ($($fmt:tt)*) => {
        eprintln!($($fmt)*);
        process::exit(1);
    }
}

macro_rules! json_fail {
    ($ep:expr, $($fmt:tt)*) => {
        eprintln!($($fmt)*);
        eprintln!("Endpoint: {}", $ep);
        eprintln!("Verify that all arguments are spelled correctly, or wait a few minutes if sending frequent requests");
        process::exit(1);
    }
}

macro_rules! pad {
    ($str:expr, $len:expr) => {
        format!("{}{}", $str, " ".repeat($len))
    }
}

const EVAL_DIGITS: usize = 5;
const NAME_LEN: usize = 8;
const VALUE_LEN: usize = 7;
const VALUE_MAX_DEC: usize = 5;
const CHANGE_LEN: usize = 7;

const EVAL_PADDING: &str = "   ";
const TOKENS_PADDING: &str = "     ";
const TOP_PADDING: &str = "   ";

const MAGNITUDE: [char; 5] = [' ', 'K', 'M', 'B', 'T'];
const HELP: &str = r#"            
Options:
    [TOKEN]              Current value of TOKEN in fiat
    [TOKEN:N]            Convert N of fiat to TOKEN
    [N:TOKEN]            Convert N of TOKEN to fiat
    -c C | --currency C  Use fiat currency C (default is "usd")
    -h | --help          Display this help message
    -t N | --top N       Top N tokens by market cap   
"#;

/*
    With the default constants, the top table looks like this:
    +-----+----------+---------+---------+
    | 1   | ABCDEFGH | 100.00K | +1.000% |
      10               10.00K    +10.00%
      100              1.00K     +100.0%
                       100.00
                       10.00
    |     |          | 1.00000 |         |
    +-----+----------+---------+---------+
    The (individual) tokens table omits the rank column
*/

fn get_data(names: Vec<&str>, c: &String) -> serde_json::Value {
    let mut list = names[0].to_string().to_lowercase();
    for name in names.iter().skip(1) {
        list += &format!(",{}", name.to_lowercase());
    }
    let client = Client::new();
    let endpoint = format!("https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={}&include_24hr_change=true", list, c);
    let data = client.get(&endpoint).header("User-Agent", "Reqwest client").send().unwrap_or_else(|e| {
        fail!("API call failed: {}", e);
    }).text().unwrap();
    let data: serde_json::Value = serde_json::from_str(&data).unwrap_or_else(|_| {
        json_fail!(&endpoint, "Error while parsing JSON, most likely a bad API response");
    });
    let change_index = format!("{}_24h_change", c);
    for name in &names { // Check errors before returning anything 
        let token = data.get(name).unwrap_or_else(|| {
            if data.to_string() == "{}" { 
                json_fail!(&endpoint, "Empty response from API"); 
            }
            if let Some(err) = data.get("error") {
                json_fail!(&endpoint, "Request failed: {}", err);
            }
            json_fail!(&endpoint, "At least one token is missing from the API response");
        }).as_object().unwrap();
        if token[c].as_f64().is_none() || token[&change_index].as_f64().is_none() {
            json_fail!(&endpoint, "Missing JSON fields in token data"); 
        }
    }
    data
}

fn format_rank(rank: u8) -> String { // Only up to 3 digits (max -t is 250)
    pad!(rank, (2 - rank.ilog10() as usize).try_into().unwrap())
}

fn format_name(name: &str, max_len: Option<usize>) -> String {
    let capitals = name.chars().fold(0, |acc, char| acc + char.is_uppercase() as usize);
    let name_str = if capitals > 0 { name.to_string() } else { 
        format!(
            "{}{}",
            name.chars().take(1).collect::<String>().to_uppercase(),
            name.chars().skip(1).collect::<String>().to_lowercase()
        ) 
    };
    let name_len = name.chars().count();
    if max_len.is_some() && name_len > max_len.unwrap() {
        return format!(
            "{}-",
            name_str.chars().take(max_len.unwrap() - 1).collect::<String>(),
        );
    }
    if max_len.is_none() { 
        name_str 
    } else { 
        pad!(name_str, max_len.unwrap() - name_len) 
    }
}

fn format_value(value: f64) -> String {
    let log10 = value.log10().floor() as isize;
    if log10 < 0 {
        return format!(
            "{:.1$}", 
            value, 
            cmp::min(VALUE_LEN - 2, VALUE_MAX_DEC)
        );
    }
    let log10 = cmp::max(log10, 0) as usize;
    let mag = log10 / 3;
    if mag > MAGNITUDE.len() - 1 { // Value is at least 1 quadrillion 
        return "PUMPED!".to_string();
    }
    let scalar = 10_usize.pow(mag as u32 * 3);
    let left = value as usize / scalar; // {left}.{right}
    let right = (value / scalar as f64 % 1.0 * 100.0 + 0.5) as usize;
    let right = if right < 10 { format!("0{}", right) } else { right.to_string() };
    let fmt = format!(
        "{}.{}{}",
        left,
        right,
        MAGNITUDE[mag],
    );
    pad!(fmt, VALUE_LEN - fmt.chars().count())
}

fn format_change(change: f64) -> String {
    if change.abs() < 0.001 {
        return format!(" {:.1$}%", f64::EPSILON, CHANGE_LEN - 4);
    }
    let log10 = (change.abs().log10().floor() as usize).try_into().unwrap_or_else(|_| 0);
    if log10 >= 3 {
        if change > 0.0 { return "PUMPED!".to_string(); }
        else { return "DUMPED!".to_string(); }
    }
    let prec = CHANGE_LEN - 4 - log10;
    /* 
        If change is a whole number, it will be missing any decimal places
        when printed; this is fixed by adding a negligibly small amount, 
        then printing it with limited precision.
    */
    let change = if change.abs() % 1.0 == 0.0 { change + f64::EPSILON } else { change };
    let change_str = format!("{:.1$}", change.abs().to_string(), prec + 2);
    format!(
        "{}{}{}%", 
        if change > 0.0 { "+" } else { "-" }, 
        change_str,
        "0".repeat(CHANGE_LEN - 2 - change_str.chars().count())
    )
}

fn print_eval_data(eval: Vec<&str>, c: &String, append_nl: bool) {
    let mut eval_tokens: Vec<EvalToken> = Vec::new();
    for arg in eval {
        let split = arg.split_once(":").unwrap();
        let ordering = (
            split.0.parse::<f64>(), 
            split.1.parse::<f64>()
        );
        match ordering {
            (Ok(amount), Err(_)) | (Err(_), Ok(amount)) => eval_tokens.push(EvalToken {
                name: if ordering.0.is_err() {
                    split.0.to_string()
                } else { 
                    split.1.to_string()
                },
                amount: amount,
                fiat_to_token: ordering.0.is_err()
            }),
            _ => { fail!("Invalid conversion syntax: {}", arg); }
        }
    }
    let tokens = eval_tokens.iter().map(|t| t.name.as_ref()).collect::<Vec<_>>();
    // The order of tokens in the API response will be different
    let mut index_map = std::collections::HashMap::new();
    for (i, token) in eval_tokens.iter().enumerate() {
        index_map.insert(token.name.to_string(), i);
    }
    let data = get_data(tokens.clone(), c);
    println!("");
    for e in &eval_tokens {
        let token_data = data.get(&e.name).unwrap().as_object().unwrap();
        let value = token_data[c].as_f64().unwrap();
        let value = if e.fiat_to_token { 
            e.amount / value
        } else {
            e.amount * value
        };
        let log10 = value.log10().floor() as isize + 1;
        let value = format!(
            "{:.1$}", 
            value, 
            cmp::max(0, EVAL_DIGITS as isize - log10) as usize
        );
        let names = if e.fiat_to_token {
            (c.to_uppercase(), format_name(&e.name, None))
        } else {
            (format_name(&e.name, None), c.to_uppercase())
        };
        println!(
            "{}{} {} -> {} {}",
            EVAL_PADDING,
            e.amount,
            names.0,
            value,
            names.1
        );
    }
    if append_nl { println!(""); }
}

fn print_token_data(tokens: Vec<&str>, c: &String, append_nl: bool) {
    let data = get_data(tokens.clone(), c);
    let change_index = format!("{}_24h_change", c);
    if tokens.len() == 1 && append_nl {
        let token_data = data.get(tokens[0]).unwrap().as_object().unwrap();
        let value = token_data[c].as_f64().unwrap();
        let change = token_data[&change_index].as_f64().unwrap();
        println!("\n{}{} {}\n", TOKENS_PADDING, format_value(value), format_change(change));
        return;
    }
        
    println!(""); 
    for name in &tokens {
        let token_data = data.get(name).unwrap().as_object().unwrap();
        let value = token_data[c].as_f64().unwrap();
        let change = token_data[&change_index].as_f64().unwrap();
        println!(
            "{}{} {} {}",
            TOKENS_PADDING, 
            format_name(name, Some(NAME_LEN)),
            format_value(value),
            format_change(change)
        );
    }
    if append_nl { println!(""); }
}

fn print_top_data(t: u8, c: &String) {
    let client = Client::new();
    let endpoint = format!("https://api.coingecko.com/api/v3/coins/markets?per_page={}&page=1&price_change_percentage=24h&vs_currency={}", t, c);
    let data = client.get(&endpoint).header("User-Agent", "Reqwest client").send().unwrap_or_else(|e| {
        fail!("API call failed: {}", e);
    }).text().unwrap();
    let data: Vec<TopToken> = serde_json::from_str(&data).unwrap_or_else(|_| {
        json_fail!(&endpoint, "Parsing JSON failed, usually caused by an error in the API call");
    });
   
    println!("");
    for token in data {
        println!(
            "{}{} {} {} {}",
            TOP_PADDING,
            format_rank(token.market_cap_rank),
            format_name(&token.name, Some(NAME_LEN)),
            format_value(token.current_price),
            format_change(token.price_change_percentage_24h)
        )                
    }
    println!("");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(|arg| arg.as_str()).collect();
    if args.len() == 0 || args.contains(&"-h") || args.contains(&"--help") {
        println!("{}", HELP);
        process::exit((args.len() != 1) as i32);
    }

    let mut flags = Flags {
        c: "usd".to_string(),
        t: 0
    };
    let mut seek: Option<&str> = None;
    let mut eval: Vec<&str> = Vec::new();
    let mut tokens: Vec<&str> = Vec::new();
    for arg in args { 
        match arg {
            "-c" | "--currency" => { 
                if seek.is_some() { fail!("Missing value for: {}", seek.unwrap()); }
                seek = Some("-c");
            },
            "-t" | "--top" => {
                if seek.is_some() { fail!("Missing value for: {}", seek.unwrap()); }
                seek = Some("-t");
            },
            _ => {
                if seek.is_none() {
                    if arg.chars().nth(0).unwrap_or_else(|| {
                        fail!("Empty option");
                    }) == '-' {
                        fail!("Unrecognized option: {}", arg);
                    }
                    if arg.contains(":") { eval.push(arg); }
                    else { tokens.push(arg); }
                    continue;
                }
                match seek.unwrap() {
                    "-c" => flags.c = arg.to_string(),
                    "-t" => {
                        flags.t = arg.parse::<u16>().unwrap_or_else(|_| {
                            fail!("Invalid -t value \"{}\"", arg); 
                        });
                        if flags.t > 250 {
                            fail!("-t cannot be greater than 250 due to API call limitations");
                        }
                    },
                    _ => panic!("Unhandled 'seek' option")
                }                
                seek = None;
            }
        }
    }
    if seek.is_some() { 
        fail!("Missing value for {}", seek.unwrap()); 
    }
    if (eval.len(), tokens.len(), flags.t) == (0, 0, 0) {
        fail!("No tokens specified, see -h");
    }
    
    if eval.len() > 0 {
        print_eval_data(eval, &flags.c, tokens.len() == 0 && flags.t == 0);
    }
    if tokens.len() > 0 { 
        print_token_data(tokens, &flags.c, flags.t == 0); 
    }
    if flags.t > 0 { 
        print_top_data(flags.t.try_into().unwrap(), &flags.c); 
    }
}
