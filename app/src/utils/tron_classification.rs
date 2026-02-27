use std::collections::HashMap;

const ZERO_ADDRESS: &str = "T9yD14Nj9j7xAB4dbGeiX9h8unkKHxuWwb";

// Tron Swap Detection
#[derive(Debug, Clone)]
pub struct SimpleTransfer {
    pub token: String,
    pub from: String,
    pub to: String,
    pub amount: u128, // 👈 بهتره numeric باشه
}

pub fn compute_net_flows(
    transfers: &[SimpleTransfer],
) -> HashMap<String, HashMap<String, i128>> {
    // address -> token -> net amount

    let mut flows: HashMap<String, HashMap<String, i128>> = HashMap::new();

    for t in transfers {

        let amount = t.amount as i128;

        // from loses
        flows
            .entry(t.from.clone())
            .or_default()
            .entry(t.token.clone())
            .and_modify(|v| *v -= amount)
            .or_insert(-amount);

        // to gains
        flows
            .entry(t.to.clone())
            .or_default()
            .entry(t.token.clone())
            .and_modify(|v| *v += amount)
            .or_insert(amount);
    }

    flows
}

pub fn detect_swaps_advanced(
    transfers: &[SimpleTransfer],
) -> Vec<(String, String, String)> {
    // returns (user, token_in, token_out)

    let flows = compute_net_flows(transfers);
    let mut swaps = Vec::new();

    for (address, token_map) in flows {

        if address == ZERO_ADDRESS {
            continue;
        }

        let mut tokens_sent = Vec::new();
        let mut tokens_received = Vec::new();

        for (token, net) in token_map {
            if net < 0 {
                tokens_sent.push(token.clone());
            }
            if net > 0 {
                tokens_received.push(token);
            }
        }

        if !tokens_sent.is_empty() && !tokens_received.is_empty() {

            for token_in in &tokens_sent {
                for token_out in &tokens_received {

                    if token_in != token_out {
                        swaps.push((
                            address.clone(),
                            token_in.clone(),
                            token_out.clone(),
                        ));
                    }
                }
            }
        }
    }

    swaps
}

pub fn detect_bridges(
    transfers: &[SimpleTransfer],
) -> Vec<(String, String, String)> {
    // returns (address, token, direction)
    // direction = "bridge_in" | "bridge_out"

    let flows = compute_net_flows(transfers);
    let mut bridges = Vec::new();

    for (address, token_map) in flows {

        if address == ZERO_ADDRESS {
            continue;
        }

        for (token, net) in token_map {

            // Mint → bridge_in
            if net > 0 &&
               transfers.iter().any(|t| 
                   t.from == ZERO_ADDRESS &&
                   t.to == address &&
                   t.token == token
               )
            {
                bridges.push((
                    address.clone(),
                    token.clone(),
                    "bridge_in".to_string(),
                ));
            }

            // Burn → bridge_out
            if net < 0 &&
               transfers.iter().any(|t|
                   t.to == ZERO_ADDRESS &&
                   t.from == address &&
                   t.token == token
               )
            {
                bridges.push((
                    address.clone(),
                    token.clone(),
                    "bridge_out".to_string(),
                ));
            }
        }
    }

    bridges
}