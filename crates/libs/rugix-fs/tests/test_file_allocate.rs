use byte_calc::NumBytes;

use rugix_blocking::{block, blocking};
use rugix_fs::{Copier, File, FsResult, TempDir};

#[tokio::test]
async fn test_allocate() {
    blocking(|cx| {
        let tempdir = block!(try TempDir::create(cx));
        let test_file = tempdir.path().join("test_file");
        for size in [1, 8, 32, 64] {
            let size = NumBytes::kibibytes(size);
            let path = test_file.clone();
            block!(try rugix_fs::allocate_file(cx, &path, size));
            let metadata = block!(try rugix_fs::read_metadata(cx, &path));
            assert_eq!(metadata.len(), size.raw);
        }
        FsResult::Done(Ok(()))
    })
    .await
    .unwrap();

    blocking(|cx| {
        let tempdir = block!(try TempDir::create(cx));
        let mut test_file = block!(try File::create(cx, &tempdir.path().join("test_file")));
        let size = NumBytes::kibibytes(64);
        block!(try test_file.allocate(NumBytes::new(0), size));
        let metadata = block!(try test_file.read_metadata());
        assert_eq!(metadata.len(), size);
        block!(try test_file.allocate(NumBytes::new(0), NumBytes::kibibytes(32)));
        let metadata = block!(try test_file.read_metadata());
        assert_eq!(metadata.len(), size);
        FsResult::Done(Ok(()))
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_copy_file_range() {
    blocking(|cx| {
        let mut copier = Copier::new();
        let tempdir = block!(try TempDir::create(cx));
        let src_path = tempdir.path().join("src_file");
        let dst_path = tempdir.path().join("dst_file");
        let mut src_file = block!(try File::create(cx, &src_path));
        let mut dst_file = block!(try File::create(cx, &dst_path));
        block!(try src_file.write(b"Hello, World!"));
        block!(try copier.copy_file_range(
            cx,
            &mut src_file,
            NumBytes::new(7),
            &mut dst_file,
            NumBytes::new(3),
            NumBytes::new(6),
        ));
        block!(try dst_file.set_current_position(NumBytes::new(0)));
        let dst_contents = block!(try dst_file.read_to_vec(None));
        assert_eq!(dst_contents, b"\0\0\0World!");
        block!(try src_file.allocate(NumBytes::new(0), NumBytes::kibibytes(64)));
        let block_two = NumBytes::kibibytes(4) * 2;
        block!(try src_file.set_current_position(block_two));
        block!(try src_file.write(b"Hello, World!"));
        block!(try copier.copy_file_range(
            cx,
            &mut src_file,
            NumBytes::new(0),
            &mut dst_file,
            NumBytes::new(0),
            NumBytes::kibibytes(64),
        ));
        block!(try src_file.set_current_position(NumBytes::new(0)));
        block!(try dst_file.set_current_position(NumBytes::new(0)));
        let src_contents = block!(try src_file.read_to_vec(None));
        let dst_contents = block!(try dst_file.read_to_vec(None));
        assert_eq!(src_contents.len(), dst_contents.len());
        assert_eq!(src_contents, dst_contents);
        FsResult::Done(Ok(()))
    })
    .await
    .unwrap();
}
