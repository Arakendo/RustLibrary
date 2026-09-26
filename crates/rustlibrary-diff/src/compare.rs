use crate::{Change, DiffError as E, Limits};

struct Work(usize);
impl Work {
    fn spend(&mut self, amount: usize) -> Result<(), E> {
        self.0 = self.0.checked_sub(amount).ok_or(E::WorkLimit)?;
        Ok(())
    }
    fn equal(&mut self, a: &str, b: &str) -> Result<bool, E> {
        self.spend(1)?;
        if a.len() != b.len() {
            return Ok(false);
        }
        self.spend(a.len())?;
        Ok(a == b)
    }
}

// Boundaries include 0 and every terminated/nonempty final line's end. Empty
// text has zero diff lines; a trailing terminator creates no phantom data line.
fn boundaries(text: &str, max_lines: usize) -> Result<Vec<usize>, E> {
    let mut out = Vec::new();
    out.try_reserve_exact(1).map_err(|_| E::AllocationFailed)?;
    out.push(0);
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let ch = bytes[at];
        at += 1;
        if ch == b'\r' && bytes.get(at) == Some(&b'\n') {
            at += 1;
        }
        if ch == b'\r' || ch == b'\n' || at == bytes.len() {
            if out.len() > max_lines {
                return Err(E::LineLimit);
            }
            out.try_reserve(1).map_err(|_| E::AllocationFailed)?;
            out.push(at);
        }
    }
    Ok(out)
}

/// Computes exact line changes with a bounded longest-common-subsequence table.
/// CR, LF and CRLF are retained in line tokens. No whitespace normalization.
/// Ties delete old lines first. Budget failure returns no partial result.
pub fn diff_lines(old: &str, new: &str, limits: Limits) -> Result<Vec<Change>, E> {
    let bytes = old.len().checked_add(new.len()).ok_or(E::InputLimit)?;
    if bytes > limits.input_bytes {
        return Err(E::InputLimit);
    }
    let mut work = Work(limits.work);
    work.spend(bytes)?;
    let a = boundaries(old, limits.lines)?;
    let n = a.len() - 1;
    let b = boundaries(new, limits.lines - n)?;
    let m = b.len() - 1;
    let width = b.len();
    let cells = a.len().checked_mul(width).ok_or(E::CellLimit)?;
    if cells > limits.cells {
        return Err(E::CellLimit);
    }
    work.spend(cells)?;
    let mut table = Vec::<usize>::new();
    table
        .try_reserve_exact(cells)
        .map_err(|_| E::AllocationFailed)?;
    table.resize(cells, 0);
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i * width + j] = if work.equal(&old[a[i]..a[i + 1]], &new[b[j]..b[j + 1]])? {
                1 + table[(i + 1) * width + j + 1]
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let mut changes = Vec::new();
    let (mut i, mut j) = (0, 0);
    let mut pending = None;
    while i < n || j < m {
        work.spend(1)?;
        if i < n && j < m && work.equal(&old[a[i]..a[i + 1]], &new[b[j]..b[j + 1]])? {
            if let Some((start_i, start_j)) = pending.take() {
                push_change(&mut changes, &a, &b, start_i..i, start_j..j, limits.changes)?;
            }
            i += 1;
            j += 1;
        } else {
            pending.get_or_insert((i, j));
            if i < n && (j == m || table[(i + 1) * width + j] >= table[i * width + j + 1]) {
                i += 1;
            } else {
                j += 1;
            }
        }
    }
    if let Some((start_i, start_j)) = pending {
        push_change(&mut changes, &a, &b, start_i..i, start_j..j, limits.changes)?;
    }
    Ok(changes)
}

fn push_change(
    out: &mut Vec<Change>,
    a: &[usize],
    b: &[usize],
    old: std::ops::Range<usize>,
    new: std::ops::Range<usize>,
    limit: usize,
) -> Result<(), E> {
    if out.len() >= limit {
        return Err(E::ChangeLimit);
    }
    out.try_reserve(1).map_err(|_| E::AllocationFailed)?;
    out.push(Change {
        old_bytes: a[old.start]..a[old.end],
        new_bytes: b[new.start]..b[new.end],
        old_lines: old,
        new_lines: new,
    });
    Ok(())
}
