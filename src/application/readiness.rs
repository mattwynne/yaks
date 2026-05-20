use std::collections::HashMap;

use crate::adapters::views::{ReadinessReasonView, ReadinessView, YakBlockerView};
use crate::domain::events::BlockerSource;
use crate::domain::slug::YakId;
use crate::domain::{Yak, YakMap, YakState};

pub fn build_readiness_views(map: &YakMap, yaks: &[Yak]) -> HashMap<YakId, ReadinessView> {
    let yaks_by_id: HashMap<YakId, Yak> = yaks
        .iter()
        .map(|yak| (yak.id.clone(), yak.clone()))
        .collect();
    yaks.iter()
        .map(|yak| (yak.id.clone(), readiness_for(map, yak, &yaks_by_id)))
        .collect()
}

fn readiness_for(map: &YakMap, yak: &Yak, yaks_by_id: &HashMap<YakId, Yak>) -> ReadinessView {
    let mut reasons = Vec::new();

    match yak.state {
        YakState::Todo => {}
        YakState::Wip => reasons.push(ReadinessReasonView {
            kind: "state".to_string(),
            message: "state is wip".to_string(),
            yak: None,
            blocker: None,
            children: vec![],
        }),
        YakState::Done => reasons.push(ReadinessReasonView {
            kind: "state".to_string(),
            message: "state is done".to_string(),
            yak: None,
            blocker: None,
            children: vec![],
        }),
        YakState::Blocked => reasons.push(ReadinessReasonView {
            kind: "state".to_string(),
            message: "state is blocked".to_string(),
            yak: None,
            blocker: None,
            children: vec![],
        }),
    }

    for blocker in map.active_blockers(&yak.id) {
        match blocker.source {
            BlockerSource::Yak(id) => {
                let blocker_view = yaks_by_id.get(&id).map(|blocking_yak| YakBlockerView {
                    kind: "yak".to_string(),
                    id: Some(blocking_yak.id.as_str().to_string()),
                    name: blocking_yak.name.to_string(),
                    state: Some(blocking_yak.state.to_string()),
                    reason: blocker.reason.clone(),
                });
                let name = blocker_view
                    .as_ref()
                    .map(|view| view.name.clone())
                    .unwrap_or_else(|| id.as_str().to_string());
                let message = match &blocker.reason {
                    Some(reason) => format!("blocked by {name}: {reason}"),
                    None => format!("blocked by {name}"),
                };
                reasons.push(ReadinessReasonView {
                    kind: "yak_blocker".to_string(),
                    message,
                    yak: Some(name),
                    blocker: blocker_view,
                    children: vec![],
                });
            }
            BlockerSource::Manual => {
                let reason = blocker
                    .reason
                    .clone()
                    .unwrap_or_else(|| "manual blocker".to_string());
                reasons.push(ReadinessReasonView {
                    kind: "manual_blocker".to_string(),
                    message: format!("blocked by manual reason: {reason}"),
                    yak: None,
                    blocker: Some(YakBlockerView {
                        kind: "manual".to_string(),
                        id: None,
                        name: "manual".to_string(),
                        state: None,
                        reason: Some(reason),
                    }),
                    children: vec![],
                });
            }
        }
    }

    let incomplete_children: Vec<String> = yaks_by_id
        .values()
        .filter(|child| child.parent_id.as_ref() == Some(&yak.id))
        .filter(|child| child.state != YakState::Done)
        .map(|child| path_for(child, yaks_by_id))
        .collect();
    if !incomplete_children.is_empty() {
        reasons.push(ReadinessReasonView {
            kind: "incomplete_children".to_string(),
            message: format!(
                "has incomplete children: {}",
                incomplete_children.join(", ")
            ),
            yak: None,
            blocker: None,
            children: incomplete_children,
        });
    }

    ReadinessView {
        ready: reasons.is_empty(),
        reasons,
    }
}

fn path_for(yak: &Yak, yaks_by_id: &HashMap<YakId, Yak>) -> String {
    let mut names = vec![yak.name.to_string()];
    let mut parent_id = yak.parent_id.clone();
    while let Some(id) = parent_id {
        let Some(parent) = yaks_by_id.get(&id) else {
            break;
        };
        names.push(parent.name.to_string());
        parent_id = parent.parent_id.clone();
    }
    names.reverse();
    names.join("/")
}
