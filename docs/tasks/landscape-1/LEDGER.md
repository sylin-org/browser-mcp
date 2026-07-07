# landscape-1 batch: LEDGER

Durable progress for the landscape-1 batch (ADR-0041/0042). One task = one commit. Update this
file at the end of every task, per BOOTSTRAP step 5. This is the single source of truth for
"where are we"; a fresh executor resumes from RESUME HERE with no other context.

## RESUME HERE

**Nothing executed yet. L1 (sources audit key) is NEXT.** Check the whole-batch STOP
preconditions in BOOTSTRAP first (ADR-0042 Accepted; the currency note present; clean tree),
then record the baseline `cargo test` count below before starting L1.

Baseline test count at batch start: (executor records)

## Log

Template per task:

```
### L<N>: <title> -- DONE (<commit>) | BLOCKED | SKIPPED
- Baseline test count -> new test count.
- What landed (2-4 sentences, concrete file names).
- Deviations: D1..Dn (or "none"). A deviation is ANY divergence from the task file or PINS,
  including renames, moved code, extra tests, or clarified wording.
```
