use std::{
    marker::PhantomData,
    ops::{Add, AddAssign},
    sync::{Arc, Mutex},
    time::Instant,
};

use hashbrown::HashMap;
use tracing::{
    Subscriber,
    field::Visit,
    span::{self, Id},
};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq)]
pub(super) struct RuleId {
    pub(super) group_name: String,
    pub(super) rule_set_name: String,
    pub(super) rule_index: usize,
    pub(super) rule_name: String,
}

type RuleMeasurements = (RuleId, Vec<(Instant, Option<Instant>)>);

#[derive(Debug, Clone)]
pub struct EvaluationLayer<S> {
    spans: Arc<Mutex<HashMap<Id, RuleVisitor>>>,
    pub spans_durations: Arc<Mutex<HashMap<Id, RuleMeasurements>>>,
    _inner: PhantomData<S>,
}

impl<S> EvaluationLayer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    pub fn new() -> Self {
        Self {
            spans: Default::default(),
            spans_durations: Default::default(),
            _inner: PhantomData,
        }
    }

    pub fn share_layer(&self) -> Self {
        Self {
            spans: self.spans.clone(),
            spans_durations: self.spans_durations.clone(),
            _inner: PhantomData,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct RuleVisitor {
    group_name: Option<String>,
    rule_set_name: Option<String>,
    rule_name: Option<String>,
    rule_index: Option<usize>,
}

impl RuleVisitor {
    fn build_id(&self) -> Option<RuleId> {
        if let Some(group_name) = &self.group_name
            && let Some(rule_set_name) = &self.rule_set_name
            && let Some(rule_name) = &self.rule_name
            && let Some(rule_index) = self.rule_index
        {
            Some(RuleId {
                group_name: group_name.clone(),
                rule_set_name: rule_set_name.clone(),
                rule_index,
                rule_name: rule_name.clone(),
            })
        } else {
            None
        }
    }
}

impl Add<&RuleVisitor> for &RuleVisitor {
    type Output = RuleVisitor;

    fn add(self, rhs: &RuleVisitor) -> Self::Output {
        Self::Output {
            group_name: self.group_name.clone().or(rhs.group_name.clone()),
            rule_set_name: self.rule_set_name.clone().or(rhs.rule_set_name.clone()),
            rule_name: self.rule_name.clone().or(rhs.rule_name.clone()),
            rule_index: self.rule_index.or(rhs.rule_index),
        }
    }
}

impl AddAssign<&RuleVisitor> for RuleVisitor {
    fn add_assign(&mut self, rhs: &RuleVisitor) {
        *self = &*self + rhs;
    }
}

impl Visit for RuleVisitor {
    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn core::fmt::Debug) {}

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "rule" {
            self.rule_name = Some(value.to_string());
        }
        if field.name() == "ruleset" {
            self.rule_set_name = Some(value.to_string());
        }
        if field.name() == "group" {
            self.group_name = Some(value.to_string());
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "index" {
            self.rule_index = Some(value as usize);
        }
    }
}

impl<S> Layer<S> for EvaluationLayer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, _ctx: Context<'_, S>) {
        let mut span_lock = self.spans.lock().unwrap();
        let visitor = span_lock.entry(id.clone()).or_default();
        attrs.values().record(visitor);
    }

    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        self.insert_rule_id_if_needed(id, ctx);
        let mut durations_lock = self.spans_durations.lock().unwrap();

        if let Some((_, durations)) = durations_lock.get_mut(id) {
            durations.push((Instant::now(), None));
        }
    }

    fn on_exit(&self, id: &span::Id, _ctx: Context<'_, S>) {
        let mut durations_lock = self.spans_durations.lock().unwrap();

        if let Some((_, durations)) = durations_lock.get_mut(id)
            && let Some((_, end)) = durations.last_mut()
        {
            let now = Instant::now();
            *end = Some(now);
        }
    }
}

impl<S> EvaluationLayer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn insert_rule_id_if_needed(&self, id: &span::Id, ctx: Context<'_, S>) {
        let mut durations_lock = self.spans_durations.lock().unwrap();

        if !durations_lock.contains_key(id) {
            let mut span_lock = self.spans.lock().unwrap();
            let first = ctx.span(id).expect("expected: span id exists in registry");

            let mut this_rule = span_lock.entry(id.clone()).or_default().clone();

            if let Some(second) = first.parent() {
                for parent in second.scope().from_root() {
                    if let Some(parent) = span_lock.get(&parent.id()) {
                        this_rule += parent;
                    }
                }
            }

            if let Some(rule_id) = this_rule.build_id() {
                durations_lock.insert(id.clone(), (rule_id, Default::default()));
            }
        }
    }
}
