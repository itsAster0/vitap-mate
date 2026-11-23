// Copied from convex 0.10.2 (https://github.com/get-convex/convex-rs), src/base_client/query_result.rs.
// Copyright 2024 Convex, Inc. Licensed under the Apache License, Version 2.0
// (http://www.apache.org/licenses/LICENSE-2.0); see NOTICE at the repo root.
// Modified for vitapmate: `ConvexValue` (JSON) instead of `convex::Value`, std's maps instead
// of imbl's.

use convex_sync_types::{QueryId, types::ErrorPayload};
use std::collections::{BTreeMap, BTreeSet};

use super::SubscriberId;
use crate::convex::value::{ConvexError, ConvexValue as Value};

/// Result of a Convex function (query/mutation/action).
///
/// The function returns a Convex value or an error message string.
#[derive(Clone, Eq, PartialEq)]
pub enum FunctionResult {
    /// The Convex value returned on a successful run of a Convex function
    Value(Value),
    /// The error message of a Convex function run that does not complete
    /// successfully.
    ErrorMessage(String),
    /// The error payload of a Convex function run that doesn't complete
    /// successfully, with an application-level error.
    ConvexError(ConvexError),
}

impl From<Result<Value, ErrorPayload<Value>>> for FunctionResult {
    fn from(result: Result<Value, ErrorPayload<Value>>) -> Self {
        match result {
            Ok(value) => FunctionResult::Value(value),
            Err(ErrorPayload::ErrorData { message, data }) => {
                FunctionResult::ConvexError(ConvexError { message, data })
            }
            Err(ErrorPayload::Message(message)) => FunctionResult::ErrorMessage(message),
        }
    }
}

impl From<FunctionResult> for Result<Value, ErrorPayload<Value>> {
    fn from(result: FunctionResult) -> Self {
        match result {
            FunctionResult::Value(value) => Ok(value),
            FunctionResult::ErrorMessage(error) => Err(ErrorPayload::Message(error)),
            FunctionResult::ConvexError(error) => Err(ErrorPayload::ErrorData {
                message: error.message,
                data: error.data,
            }),
        }
    }
}

impl std::fmt::Debug for FunctionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionResult::Value(value) => f.debug_tuple("Value").field(value).finish(),
            FunctionResult::ErrorMessage(error) => write!(f, "{error}"),
            FunctionResult::ConvexError(error) => {
                f.debug_tuple("ConvexError").field(error).finish()
            }
        }
    }
}

/// A mapping from [`SubscriberId`] to its current result [`FunctionResult`]
/// for each actively subscribed query.
#[derive(Clone, Default, Debug)]
pub struct QueryResults {
    pub(super) results: BTreeMap<QueryId, FunctionResult>,
    pub(super) subscribers: BTreeSet<SubscriberId>,
}

impl QueryResults {
    /// Get the [`FunctionResult`] for the given [`SubscriberId`]
    pub fn get(&self, subscriber_id: &SubscriberId) -> Option<&FunctionResult> {
        if !self.subscribers.contains(subscriber_id) {
            return None;
        };
        self.results.get(&subscriber_id.0)
    }

    /// Get the size of the map.
    pub fn len(&self) -> usize {
        self.subscribers.len()
    }

    /// Test whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.subscribers.is_empty()
    }

    /// Get an iterator over the subscriber_id/query_result pairs of the map.
    pub fn iter(&self) -> impl Iterator<Item = (&SubscriberId, Option<&FunctionResult>)> {
        self.subscribers.iter().map(|s| (s, self.results.get(&s.0)))
    }
}

#[cfg(test)]
mod tests {
    use convex_sync_types::QueryId;
    use std::collections::{BTreeMap, BTreeSet};

    use serde_json::json;

    use crate::convex::{
        base_client::{FunctionResult, QueryResults, SubscriberId},
        value::ConvexValue as Value,
    };

    #[test]
    fn test_query_results() {
        let q = QueryId::new;
        let s = SubscriberId;

        let qr = QueryResults {
            results: BTreeMap::from([
                (q(0), FunctionResult::Value(Value(json!(null)))),
                (q(1), FunctionResult::Value(Value(json!(5.0)))),
            ]),
            subscribers: BTreeSet::from([s(q(0), 0), s(q(0), 1), s(q(1), 0), s(q(2), 0)]),
        };
        assert_eq!(
            qr.get(&s(q(0), 0)),
            Some(&FunctionResult::Value(Value(json!(null))))
        );
        assert_eq!(
            qr.get(&s(q(0), 1)),
            Some(&FunctionResult::Value(Value(json!(null))))
        );
        assert_eq!(
            qr.get(&s(q(1), 0)),
            Some(&FunctionResult::Value(Value(json!(5.0))))
        );
        assert_eq!(qr.get(&s(q(2), 0)), None,);
        assert_eq!(qr.len(), 4);
        assert!(!qr.is_empty());
        let v: Vec<_> = qr.iter().collect();
        assert_eq!(
            v,
            vec![
                (
                    &s(q(0), 0),
                    Some(&FunctionResult::Value(Value(json!(null))))
                ),
                (
                    &s(q(0), 1),
                    Some(&FunctionResult::Value(Value(json!(null))))
                ),
                (&s(q(1), 0), Some(&FunctionResult::Value(Value(json!(5.0))))),
                (&s(q(2), 0), None),
            ],
        );
    }
}
