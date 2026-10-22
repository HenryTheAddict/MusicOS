//! Small read-only listening summary for the personal desktop home.
use serde::Serialize;
#[derive(Debug, Serialize, PartialEq)]
pub struct Played {
    pub id: i64,
    pub plays: i32,
}
#[derive(Debug, Serialize, PartialEq)]
pub struct Activity {
    pub frequent: Vec<Played>,
    pub recent: Vec<i64>,
}
pub fn summarize(rows: impl IntoIterator<Item = (i64, i32, Option<i64>)>) -> Activity {
    let rows = rows.into_iter().collect::<Vec<_>>();
    let mut frequent = rows
        .iter()
        .filter(|(_, plays, _)| *plays > 0)
        .map(|(id, plays, _)| Played {
            id: *id,
            plays: *plays,
        })
        .collect::<Vec<_>>();
    frequent.sort_by_key(|r| (std::cmp::Reverse(r.plays), r.id));
    frequent.truncate(200);
    let mut recent = rows
        .iter()
        .filter(|(_, _, at)| at.is_some_and(|at| at > 0))
        .collect::<Vec<_>>();
    recent.sort_by_key(|(id, _, at)| (std::cmp::Reverse(*at), *id));
    Activity {
        frequent,
        recent: recent.into_iter().take(20).map(|(id, _, _)| *id).collect(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_and_recency_have_separate_stable_rankings() {
        let data = summarize([
            (1, 30, Some(10)),
            (2, 2, Some(30)),
            (3, 30, None),
            (4, 0, Some(0)),
        ]);
        assert_eq!(
            data.frequent,
            vec![
                Played { id: 1, plays: 30 },
                Played { id: 3, plays: 30 },
                Played { id: 2, plays: 2 }
            ]
        );
        assert_eq!(data.recent, vec![2, 1]);
        assert!(summarize([]).frequent.is_empty());
    }
    #[test]
    fn large_histories_are_bounded() {
        let data = summarize((1..=1000).map(|id| (id, id as i32, Some(id))));
        assert_eq!(data.frequent.len(), 200);
        assert_eq!(data.recent.len(), 20);
        assert_eq!(data.frequent[0].id, 1000);
    }
}
