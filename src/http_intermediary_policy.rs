//! Shared HTTP intermediary control policy used by every Pingora delivery adapter.
//!
//! RFC 9110 `Max-Forwards` is gateway transport policy, not product routing or authorization.
//! Keeping parsing and decrement semantics here prevents generic and migration composition roots
//! from drifting while each adapter retains its own admission and response-delivery mechanics.

use pingora::prelude::{Error, ErrorType, RequestHeader};

/// Largest forwarded hop budget emitted by the gateway after decrementing an admitted value.
pub(crate) const MAX_SUPPORTED_MAX_FORWARDS: u32 = 255;

/// Intermediary action derived from an admitted request's `Max-Forwards` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaxForwardsAction {
    /// The request method does not assign intermediary semantics to `Max-Forwards`, or no field exists.
    Ignore,
    /// This gateway is the final recipient because the hop budget is zero.
    FinalRecipient,
    /// Forward after replacing the field with this decremented bounded value.
    Forward(u32),
}

/// Builds the stable client-visible error for an invalid TRACE/OPTIONS hop budget.
fn invalid_max_forwards() -> Box<Error> {
    Error::explain(
        ErrorType::HTTPStatus(400),
        "TRACE/OPTIONS Max-Forwards must contain exactly one decimal value",
    )
}

/// Classifies RFC 9110 `Max-Forwards` without mutating the received request.
///
/// Arbitrarily long decimal values use saturating arithmetic because only the bounded forwarded
/// value matters. Malformed or duplicate fields fail closed before upstream selection.
pub(crate) fn max_forwards_action(request: &RequestHeader) -> pingora::Result<MaxForwardsAction> {
    if !matches!(request.method.as_str(), "TRACE" | "OPTIONS") {
        return Ok(MaxForwardsAction::Ignore);
    }

    let mut values = request.headers.get_all("max-forwards").iter();
    let Some(value) = values.next() else {
        return Ok(MaxForwardsAction::Ignore);
    };
    if values.next().is_some() {
        return Err(invalid_max_forwards());
    }

    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return Err(invalid_max_forwards());
    }
    if bytes.iter().all(|byte| *byte == b'0') {
        return Ok(MaxForwardsAction::FinalRecipient);
    }

    let received = bytes.iter().fold(0_u32, |current, digit| {
        current
            .saturating_mul(10)
            .saturating_add(u32::from(*digit - b'0'))
    });
    Ok(MaxForwardsAction::Forward(
        received
            .saturating_sub(1)
            .min(MAX_SUPPORTED_MAX_FORWARDS),
    ))
}

/// Rewrites a forwarding-eligible TRACE/OPTIONS hop budget immediately before proxy delivery.
///
/// A final-recipient result is an invariant violation at this phase because request admission should
/// already have terminated it locally. Returning 501 keeps delivery fail-closed if callback ordering
/// drifts rather than forwarding a zero-hop request.
pub(crate) fn apply_max_forwards_before_forward(
    request: &mut RequestHeader,
) -> pingora::Result<()> {
    match max_forwards_action(request)? {
        MaxForwardsAction::Ignore => Ok(()),
        MaxForwardsAction::Forward(value) => {
            request.insert_header("Max-Forwards", value.to_string())?;
            Ok(())
        }
        MaxForwardsAction::FinalRecipient => Err(Error::explain(
            ErrorType::HTTPStatus(501),
            "TRACE/OPTIONS Max-Forwards budget exhausted at gateway",
        )),
    }
}
