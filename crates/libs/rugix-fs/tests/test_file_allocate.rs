use byte_calc::NumBytes;

use rugix_fs::{Copier, File, FsResult, TempDir};
use rugix_tasks::run_blocking_unchecked;
use rugix_try::xtry;

#[test]
fn test_allocate() {
    run_blocking_unchecked(|cx| -> FsResult<()> {
        let tempdir = xtry!(xtry!(TempDir::create(cx)));
        let test_file = tempdir.path().join("test_file");
        for size in [1, 8, 32, 64] {
            let size = NumBytes::kibibytes(size);
            let path = test_file.clone();
            xtry!(xtry!(rugix_fs::allocate_file(cx, &path, size)));
            let metadata = xtry!(xtry!(rugix_fs::read_metadata(cx, &path)));
            assert_eq!(metadata.len(), size.raw);
        }
        FsResult::Done(Ok(()))
    })
    .unwrap()
    .unwrap();

    run_blocking_unchecked(|cx| -> FsResult<()> {
        let tempdir = xtry!(xtry!(TempDir::create(cx)));
        let mut test_file = xtry!(xtry!(File::create(cx, &tempdir.path().join("test_file"))));
        let size = NumBytes::kibibytes(64);
        xtry!(xtry!(test_file.allocate(NumBytes::new(0), size)));
        let metadata = xtry!(xtry!(test_file.read_metadata()));
        assert_eq!(metadata.len(), size);

        xtry!(xtry!(
            test_file.allocate(NumBytes::new(0), NumBytes::kibibytes(32))
        ));
        let metadata = xtry!(xtry!(test_file.read_metadata()));
        assert_eq!(metadata.len(), size);
        FsResult::Done(Ok(()))
    })
    .unwrap()
    .unwrap();
}

#[test]
fn test_copy_file_range() {
    run_blocking_unchecked(|cx| -> FsResult<()> {
        let mut copier = Copier::new();
        let tempdir = xtry!(xtry!(TempDir::create(cx)));
        let src_path = tempdir.path().join("src_file");
        let dst_path = tempdir.path().join("dst_file");
        let mut src_file = xtry!(xtry!(File::create(cx, &src_path)));
        let mut dst_file = xtry!(xtry!(File::create(cx, &dst_path)));
        xtry!(xtry!(src_file.write(b"Hello, World!")));
        xtry!(xtry!(copier.copy_file_range(
            cx,
            &mut src_file,
            NumBytes::new(7),
            &mut dst_file,
            NumBytes::new(3),
            NumBytes::new(6),
        )));
        xtry!(xtry!(dst_file.set_current_position(NumBytes::new(0))));
        let dst_contents = xtry!(xtry!(dst_file.read_to_vec(None)));
        assert_eq!(dst_contents, b"\0\0\0World!");
        xtry!(xtry!(
            src_file.allocate(NumBytes::new(0), NumBytes::kibibytes(64))
        ));
        let block_two = NumBytes::kibibytes(4) * 2;
        xtry!(xtry!(src_file.set_current_position(block_two)));
        xtry!(xtry!(src_file.write(b"Hello, World!")));
        xtry!(xtry!(copier.copy_file_range(
            cx,
            &mut src_file,
            NumBytes::new(0),
            &mut dst_file,
            NumBytes::new(0),
            NumBytes::kibibytes(64),
        )));
        xtry!(xtry!(src_file.set_current_position(NumBytes::new(0))));
        xtry!(xtry!(dst_file.set_current_position(NumBytes::new(0))));
        let src_contents = xtry!(xtry!(src_file.read_to_vec(None)));
        let dst_contents = xtry!(xtry!(dst_file.read_to_vec(None)));
        assert_eq!(src_contents.len(), dst_contents.len());
        assert_eq!(src_contents, dst_contents);
        FsResult::Done(Ok(()))
    })
    .unwrap()
    .unwrap();
}
