//! Detects when iRacing's SessionNum changes (Practice → Qualify → Race, etc.)
//! without relying on SessionState enum constants that are not defined in this codebase.

/// Emits the previous SessionNum once when SessionNum changes; arms silently on first call.
#[derive(Debug, Default)]
pub struct SessionTransitionDetector {
    last: Option<i32>,
}

impl SessionTransitionDetector {
    /// Call once per standings tick with the current SessionNum.
    /// Returns `Some(prev_session_num)` exactly once when the number changes.
    pub fn observe(&mut self, session_num: i32) -> Option<i32> {
        match self.last {
            Some(prev) if prev != session_num => {
                self.last = Some(session_num);
                Some(prev)
            }
            None => {
                self.last = Some(session_num);
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arms_on_first_call() {
        let mut d = SessionTransitionDetector::default();
        assert_eq!(d.observe(0), None);
    }

    #[test]
    fn emits_prev_on_change() {
        let mut d = SessionTransitionDetector::default();
        d.observe(0);
        assert_eq!(d.observe(0), None, "no change");
        assert_eq!(d.observe(1), Some(0), "transition 0→1");
        assert_eq!(d.observe(1), None, "still 1");
        assert_eq!(d.observe(2), Some(1), "transition 1→2");
    }

    #[test]
    fn repeated_same_no_emit() {
        let mut d = SessionTransitionDetector::default();
        d.observe(5);
        for _ in 0..100 {
            assert_eq!(d.observe(5), None);
        }
    }
}
