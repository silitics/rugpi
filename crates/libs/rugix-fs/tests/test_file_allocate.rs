use byte_calc::NumBytes;

use rugix_fs::{Copier, File, FsResult, TempDir};
use rugix_tasks::spawn_blocking;

#[test]
fn test_allocate() {
    spawn_blocking(|| -> FsResult<()> {
        let tempdir = TempDir::create()?;
        let test_file = tempdir.path().join("test_file");
        for size in [1, 8, 32, 64] {
            let size = NumBytes::kibibytes(size);
            let path = test_file.clone();
            rugix_fs::allocate_file(&path, size)?;
            let metadata = rugix_fs::read_metadata(&path)?;
            assert_eq!(metadata.len(), size.raw);
        }
        Ok(())
    })
    .join_blocking()
    .unwrap();

    spawn_blocking(|| -> FsResult<()> {
        let tempdir = TempDir::create()?;
        let mut test_file = File::create(&tempdir.path().join("test_file"))?;
        let size = NumBytes::kibibytes(64);
        test_file.allocate(NumBytes::new(0), size)?;
        let metadata = test_file.read_metadata()?;
        assert_eq!(metadata.len(), size);
        test_file.allocate(NumBytes::new(0), NumBytes::kibibytes(32))?;
        let metadata = test_file.read_metadata()?;
        assert_eq!(metadata.len(), size);
        Ok(())
    })
    .join_blocking()
    .unwrap();
}

#[test]
fn test_copy_file_range() {
    spawn_blocking(|| -> FsResult<()> {
        let mut copier = Copier::new();
        let tempdir = TempDir::create()?;
        let src_path = tempdir.path().join("src_file");
        let dst_path = tempdir.path().join("dst_file");
        let mut src_file = File::create(&src_path)?;
        let mut dst_file = File::create(&dst_path)?;
        src_file.write(b"Hello, World!")?;
        copier.copy_file_range(
            &mut src_file,
            NumBytes::new(7),
            &mut dst_file,
            NumBytes::new(3),
            NumBytes::new(6),
        )?;
        dst_file.set_current_position(NumBytes::new(0))?;
        let dst_contents = dst_file.read_to_vec(None)?;
        assert_eq!(dst_contents, b"\0\0\0World!");
        src_file.allocate(NumBytes::new(0), NumBytes::kibibytes(64))?;
        let block_two = NumBytes::kibibytes(4) * 2;
        src_file.set_current_position(block_two)?;
        src_file.write(b"Hello, World")?;
        copier.copy_file_range(
            &mut src_file,
            NumBytes::new(0),
            &mut dst_file,
            NumBytes::new(0),
            NumBytes::kibibytes(64),
        )?;
        src_file.set_current_position(NumBytes::new(0))?;
        dst_file.set_current_position(NumBytes::new(0))?;
        let src_contents = src_file.read_to_vec(None)?;
        let dst_contents = dst_file.read_to_vec(None)?;
        assert_eq!(src_contents.len(), dst_contents.len());
        assert_eq!(src_contents, dst_contents);
        Ok(())
    })
    .join_blocking()
    .unwrap();
}
