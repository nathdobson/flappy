use crate::error::Error;
use log::info;
use picoboot::{Access, Picoboot};
use std::fmt::{Display, Formatter};
use std::iter::Step;
use std::ops::{Add, Range};
use std::time::Duration;

#[derive(Copy, Clone, Debug)]
pub enum FlashFirmwareProgress {
    ResetInterface,
    DisableMassStorage,
    DisableXip,
    Erase(f32),
    Write(f32),
    Verify(f32),
    Reboot,
}

impl Display for FlashFirmwareProgress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut percent = None;
        match self {
            FlashFirmwareProgress::ResetInterface => {
                write!(f, "resetting interface...")?;
            }
            FlashFirmwareProgress::DisableMassStorage => {
                write!(f, "disabling mass storage...")?;
            }
            FlashFirmwareProgress::DisableXip => {
                write!(f, "disabling XIP...")?;
            }
            FlashFirmwareProgress::Erase(p) => {
                write!(f, "erasing flash")?;
                percent = Some(*p);
            }
            FlashFirmwareProgress::Write(p) => {
                write!(f, "writing flash")?;
                percent = Some(*p);
            }
            FlashFirmwareProgress::Verify(p) => {
                write!(f, "verifying flash")?;
                percent = Some(*p);
            }
            FlashFirmwareProgress::Reboot => {
                write!(f, "Rebooting...")?;
            }
        }
        if let Some(percent) = percent {
            write!(f, " ({}%)", (100.0 * percent).floor() as usize)?;
        }
        Ok(())
    }
}

struct WithProgress<I> {
    iter: I,
    count: usize,
    total: usize,
}

impl<I: ExactSizeIterator> WithProgress<I> {
    pub fn new(iter: I) -> Self {
        let total = iter.len();
        WithProgress {
            iter,
            count: 0,
            total,
        }
    }
}

impl<I: Iterator> Iterator for WithProgress<I> {
    type Item = (f32, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        let progress = (self.count as f32) / (self.total as f32);
        self.count += 1;
        let item = self.iter.next()?;
        Some((progress, item))
    }
}

fn range_chunks<T: Step + Add<Output = T> + Ord + Copy + 'static>(
    range: Range<T>,
    size: T,
) -> impl ExactSizeIterator + Iterator<Item = Range<T>>
where
    Range<T>: ExactSizeIterator<Item = T>,
    usize: TryFrom<T>,
{
    let end = range.end;
    range
        .step_by(usize::try_from(size).ok().unwrap())
        .map(move |i| i..((i + size).min(end)))
}

#[test]
fn test_range_chunks() {
    assert_eq!(
        Vec::<Range<u32>>::new(),
        range_chunks::<u32>(0..0, 1).collect::<Vec<Range<u32>>>()
    );
    assert_eq!(
        vec![0..1],
        range_chunks::<u32>(0..1, 1).collect::<Vec<Range<u32>>>()
    );
    assert_eq!(
        vec![0..1, 1..2],
        range_chunks::<u32>(0..2, 1).collect::<Vec<Range<u32>>>()
    );
    assert_eq!(
        vec![0..1, 1..2, 2..3],
        range_chunks::<u32>(0..3, 1).collect::<Vec<Range<u32>>>()
    );
    assert_eq!(
        Vec::<Range<u32>>::new(),
        range_chunks::<u32>(0..0, 2).collect::<Vec<Range<u32>>>()
    );
    assert_eq!(
        vec![0..1],
        range_chunks::<u32>(0..1, 2).collect::<Vec<Range<u32>>>()
    );
    assert_eq!(
        vec![0..2],
        range_chunks::<u32>(0..2, 2).collect::<Vec<Range<u32>>>()
    );
    assert_eq!(
        vec![0..2, 2..3],
        range_chunks::<u32>(0..3, 2).collect::<Vec<Range<u32>>>()
    );
    assert_eq!(
        vec![0..2, 2..4],
        range_chunks::<u32>(0..4, 2).collect::<Vec<Range<u32>>>()
    );
    assert_eq!(
        vec![0..2, 2..4, 4..5],
        range_chunks::<u32>(0..5, 2).collect::<Vec<Range<u32>>>()
    );
}

#[test]
fn test_with_progress() {
    assert_eq!(
        Vec::<(f32, ())>::new(),
        WithProgress::new(Vec::<()>::new().into_iter()).collect::<Vec<_>>()
    );
    assert_eq!(
        vec![(0.0, 1)],
        WithProgress::new(vec![1].into_iter()).collect::<Vec<_>>()
    );
    assert_eq!(
        vec![(0.0, 1), (0.5, 2)],
        WithProgress::new(vec![1, 2].into_iter()).collect::<Vec<_>>()
    );
    assert_eq!(
        vec![(0.0, 1), (0.33333334, 2), (0.6666667, 3)],
        WithProgress::new(vec![1, 2, 3].into_iter()).collect::<Vec<_>>()
    );
}

pub async fn flash_firmware<E>(
    conn: &mut picoboot::Connection,
    binary: &[u8],
    progress: &mut dyn FnMut(FlashFirmwareProgress) -> Result<(), E>,
) -> Result<Result<(), E>, Error> {
    if let Err(e) = progress(FlashFirmwareProgress::ResetInterface) {
        return Ok(Err(e));
    }
    conn.reset_interface().await?;
    if let Err(e) = progress(FlashFirmwareProgress::DisableMassStorage) {
        return Ok(Err(e));
    }
    conn.set_exclusive_access(Access::ExclusiveAndEject).await?;
    if let Err(e) = progress(FlashFirmwareProgress::DisableXip) {
        return Ok(Err(e));
    }
    conn.exit_xip().await?;
    let flash_start = conn.target().flash_start();
    let sector_size = conn.target().flash_sector_size();
    for (p, chunk) in WithProgress::new(range_chunks(0u32..binary.len() as u32, sector_size)) {
        if let Err(e) = progress(FlashFirmwareProgress::Erase(p)) {
            return Ok(Err(e));
        }
        conn.flash_erase(chunk.start + flash_start, sector_size)
            .await?;
    }
    for (p, chunk) in WithProgress::new(range_chunks(0u32..binary.len() as u32, sector_size)) {
        if let Err(e) = progress(FlashFirmwareProgress::Write(p)) {
            return Ok(Err(e));
        }
        conn.flash_write(
            chunk.start + flash_start,
            &binary[chunk.start as usize..chunk.end as usize],
        )
        .await?;
    }
    let mut verified = Vec::with_capacity(binary.len());
    for (p, chunk) in WithProgress::new(range_chunks(0u32..binary.len() as u32, sector_size)) {
        if let Err(e) = progress(FlashFirmwareProgress::Verify(p)) {
            return Ok(Err(e));
        }
        verified.extend_from_slice(
            &conn
                .read(chunk.start + flash_start, chunk.end - chunk.start)
                .await?,
        );
    }
    if binary != verified {
        return Err(Error::FlashVerifyError);
    }
    progress(FlashFirmwareProgress::Reboot);
    conn.reboot(Duration::from_millis(500)).await?;
    Ok(Ok(()))
}
