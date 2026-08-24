//! Execution engine bits.

use crate::api::Match;
use crate::insn::CompiledRegex;
use crate::position::PositionType;

/// A trait for finding the next match in a regex.
/// This is broken out from Executor to avoid needing to thread lifetimes
/// around.
pub trait MatchProducer: core::fmt::Debug {
    /// The position type of our indexer.
    type Position: PositionType;

    /// \return an initial position for the given start offset.
    fn initial_position(&self, offset: usize) -> Option<Self::Position>;

    /// Attempt to match at the given location.
    /// \return either the Match and the position to start looking for the next
    /// match, or None on failure.
    fn next_match(
        &mut self,
        pos: Self::Position,
        next_start: &mut Option<Self::Position>,
    ) -> Option<Match>;

    /// Reports whether matching stopped because a configured resource limit was
    /// exhausted rather than because the regular expression had no match.
    fn resource_limit_exceeded(&self) -> bool {
        false
    }
}

/// A trait for executing a regex.
pub trait Executor<'r, 't>: MatchProducer {
    /// The ASCII variant.
    type AsAscii: Executor<'r, 't>;

    /// Construct a new Executor.
    fn new(re: &'r CompiledRegex, text: &'t str) -> Self;
}

/// A struct which enables iteration over matches.
#[derive(Debug)]
pub struct Matches<Producer: MatchProducer> {
    mp: Producer,
    position: Option<Producer::Position>,
}

impl<Producer: MatchProducer> Matches<Producer> {
    pub fn new(mp: Producer, start: usize) -> Self {
        let position = mp.initial_position(start);
        Matches { mp, position }
    }

    pub fn resource_limit_exceeded(&self) -> bool {
        self.mp.resource_limit_exceeded()
    }
}

impl<Producer: MatchProducer> Iterator for Matches<Producer> {
    type Item = Match;
    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.position?;
        self.mp.next_match(pos, &mut self.position)
    }
}
