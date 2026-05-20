use std::collections::HashMap;

use crate::adapters::views::{ReadinessReasonView, ReadinessView, YakBlockerView};
use crate::domain::slug::YakId;
use crate::domain::yak_map::{MANUAL_BLOCKER_FIELD, MIGRATED_BLOCKED_REASON};
use crate::domain::{Yak, YakBlockerSnapshot, YakState};

pub fn build_readiness_views(
    yaks: &[Yak],
    yak_blockers: &[YakBlockerSnapshot],
) -> HashMap<YakId, ReadinessView> {
    let yaks_by_id: HashMap<YakId, Yak> = yaks
        .iter()
        .map(|yak| (yak.id.clone(), yak.clone()))
        .collect();
    let blockers_by_target = blockers_by_target(yak_blockers);

    yaks.iter()
        .map(|yak| {
            (
                yak.id.clone(),
                readiness_for(yak, &yaks_by_id, &blockers_by_target),
            )
        })
        .collect()
}

fn blockers_by_target(
    yak_blockers: &[YakBlockerSnapshot],
) -> HashMap<YakId, Vec<YakBlockerSnapshot>> {
    let mut result: HashMap<YakId, Vec<YakBlockerSnapshot>> = HashMap::new();
    for blocker in yak_blockers {
        result
            .entry(blocker.target.clone())
            .or_default()
            .push(blocker.clone());
    }
    for blockers in result.values_mut() {
        blockers.sort_by(|a, b| a.blocker.as_str().cmp(b.blocker.as_str()));
    }
    result
}

fn readiness_for(
    yak: &Yak,
    yaks_by_id: &HashMap<YakId, Yak>,
    blockers_by_target: &HashMap<YakId, Vec<YakBlockerSnapshot>>,
) -> ReadinessView {
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
        YakState::Blocked => {}
    }

    for blocker in blockers_by_target.get(&yak.id).into_iter().flatten() {
        let blocker_view = yaks_by_id
            .get(&blocker.blocker)
            .map(|blocking_yak| YakBlockerView {
                kind: "yak".to_string(),
                id: Some(blocking_yak.id.as_str().to_string()),
                name: blocking_yak.name.to_string(),
                state: Some(blocking_yak.state.to_string()),
                reason: blocker.reason.clone(),
            });
        let name = blocker_view
            .as_ref()
            .map(|view| view.name.clone())
            .unwrap_or_else(|| blocker.blocker.as_str().to_string());
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

    if let Some(reason) = manual_blocker_reason(yak) {
        reasons.push(manual_blocker_reason_view(reason));
    }

    let mut incomplete_children: Vec<String> = yaks_by_id
        .values()
        .filter(|child| child.parent_id.as_ref() == Some(&yak.id))
        .filter(|child| child.state != YakState::Done)
        .map(|child| path_for(child, yaks_by_id))
        .collect();
    incomplete_children.sort();
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

fn manual_blocker_reason_view(reason: String) -> ReadinessReasonView {
    ReadinessReasonView {
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
    }
}

fn manual_blocker_reason(yak: &Yak) -> Option<String> {
    if yak.state == YakState::Blocked {
        return Some(MIGRATED_BLOCKED_REASON.to_string());
    }

    yak.fields
        .get(MANUAL_BLOCKER_FIELD)
        .map(|reason| reason.trim())
        .filter(|reason| !reason.is_empty())
        .map(str::to_string)
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
