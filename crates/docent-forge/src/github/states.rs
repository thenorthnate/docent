//! Whether PRs are open, closed or merged: one GraphQL query per 100 PRs on
//! a host, each repo under its own alias.

use std::collections::{BTreeMap, HashMap};

use super::{client, graphql_base};
use crate::{ConnectError, Host, PullRef, PullState, Token};

/// PRs per query, to keep each well inside GitHub's query limits.
const MAX_PULLS: usize = 100;

/// The query for `repos`, each with its PR numbers. Repo `i` is aliased
/// `r{i}` and named by the variables `o{i}` and `n{i}`, so its owner and
/// name never enter the query text; PR numbers are integers, so they can.
fn query(repos: &[(&str, &str, Vec<u64>)]) -> String {
    let mut params = Vec::new();
    let mut fields = String::new();
    for (i, (_, _, numbers)) in repos.iter().enumerate() {
        params.push(format!("$o{i}: String!, $n{i}: String!"));
        fields.push_str(&format!("r{i}: repository(owner: $o{i}, name: $n{i}) {{ "));
        for number in numbers {
            fields.push_str(&format!(
                "p{number}: pullRequest(number: {number}) {{ state }} "
            ));
        }
        fields.push_str("} ");
    }
    format!("query({}) {{ {fields}}}", params.join(", "))
}

#[derive(serde::Serialize)]
struct Request {
    query: String,
    variables: HashMap<String, String>,
}

/// A repo's PRs by alias. A PR GitHub couldn't resolve is `null`.
type RepoPulls = HashMap<String, Option<Pull>>;

#[derive(serde::Deserialize)]
struct Response {
    /// `null` when the whole query failed. A repo or PR GitHub couldn't
    /// resolve is `null` under its alias.
    data: Option<HashMap<String, Option<RepoPulls>>>,
}

#[derive(serde::Deserialize)]
struct Pull {
    state: String,
}

fn state(state: &str) -> Option<PullState> {
    match state {
        "OPEN" => Some(PullState::Open),
        "CLOSED" => Some(PullState::Closed),
        "MERGED" => Some(PullState::Merged),
        // A state GitHub adds later says nothing Docent can act on.
        _ => None,
    }
}

/// The states of `pulls`, all on `host`, in the order given. Any query that
/// fails fails the lot, so a failure never passes for an answer.
pub(crate) async fn pull_states(
    host: &Host,
    api_base: &str,
    token: &Token,
    pulls: &[&PullRef],
) -> Result<Vec<(PullRef, PullState)>, ConnectError> {
    let client = client(host, graphql_base(api_base), token)?;
    let mut found: HashMap<(&str, &str, u64), PullState> = HashMap::new();
    for batch in pulls.chunks(MAX_PULLS) {
        let mut by_repo: BTreeMap<(&str, &str), Vec<u64>> = BTreeMap::new();
        for pull in batch {
            by_repo
                .entry((&pull.owner, &pull.repo))
                .or_default()
                .push(pull.number);
        }
        let repos: Vec<(&str, &str, Vec<u64>)> = by_repo
            .into_iter()
            .map(|((owner, name), numbers)| (owner, name, numbers))
            .collect();
        let mut variables = HashMap::new();
        for (i, (owner, name, _)) in repos.iter().enumerate() {
            variables.insert(format!("o{i}"), owner.to_string());
            variables.insert(format!("n{i}"), name.to_string());
        }
        let request = Request {
            query: query(&repos),
            variables,
        };
        let response: Response = client
            .post("/graphql", Some(&request))
            .await
            .map_err(|err| super::api_error(host, err).into_connect(host))?;
        let Some(mut data) = response.data else {
            return Err(ConnectError::Unreachable {
                host: host.as_str().to_string(),
                reason: "GitHub answered without the PRs' states".to_string(),
            });
        };
        // Only the aliases Docent asked for are read, so nothing else in the
        // answer can name a PR.
        for (i, (owner, name, numbers)) in repos.iter().enumerate() {
            let Some(Some(mut repo)) = data.remove(&format!("r{i}")) else {
                continue;
            };
            for &number in numbers {
                let pull = repo.remove(&format!("p{number}")).flatten();
                if let Some(state) = pull.and_then(|pull| state(&pull.state)) {
                    found.insert((owner, name, number), state);
                }
            }
        }
    }
    Ok(pulls
        .iter()
        .filter_map(|pull| {
            let state = found.get(&(pull.owner.as_str(), pull.repo.as_str(), pull.number))?;
            Some(((*pull).clone(), *state))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owners_and_names_go_in_variables_and_numbers_in_the_query() {
        let q = query(&[("evil\") {", "x", vec![1, 22]), ("a", "b", vec![3])]);

        assert_eq!(
            q,
            "query($o0: String!, $n0: String!, $o1: String!, $n1: String!) { \
             r0: repository(owner: $o0, name: $n0) { \
             p1: pullRequest(number: 1) { state } p22: pullRequest(number: 22) { state } } \
             r1: repository(owner: $o1, name: $n1) { p3: pullRequest(number: 3) { state } } }"
        );
    }
}
