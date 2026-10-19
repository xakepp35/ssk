use std::collections::BTreeSet;
/// Virtual q×q board, at most two slots per physical participant.
pub fn neighbours(members: &[String], id: &str) -> BTreeSet<String> {
    let Some(index) = members.iter().position(|p| p == id) else {
        return BTreeSet::new();
    };
    let n = members.len();
    if n <= crate::constants::FULL_MESH_PEERS {
        return members
            .iter()
            .filter(|p| p.as_str() != id)
            .cloned()
            .collect();
    }
    let q = (n as f64).sqrt().ceil() as usize;
    let mut out = BTreeSet::new();
    for a in (index..q * q).step_by(n) {
        for b in 0..q * q {
            if a / q == b / q || a % q == b % q || (a / q + a % q) % q == (b / q + b % q) % q {
                if members[b % n] != id {
                    out.insert(members[b % n].clone());
                }
            }
        }
    }
    out
}
pub fn intermediaries(members: &[String], a: &str, b: &str) -> Vec<String> {
    neighbours(members, a)
        .intersection(&neighbours(members, b))
        .cloned()
        .collect()
}
