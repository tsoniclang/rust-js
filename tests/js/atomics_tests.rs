use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use tsonic_rust_js::{atomics, ArrayBuffer, DataView, Int32Array};

#[test]
fn atomic_wait_validates_backing_bounds_conversion_and_timeout() {
    let buffer = ArrayBuffer::new_shared(12.0).unwrap();
    let values = Int32Array::from_buffer_offset(buffer.clone(), 4.0).unwrap();
    assert_eq!(
        atomics::wait(&values, 0.0, 1.0, 1000.0).unwrap(),
        "not-equal"
    );
    assert_eq!(atomics::wait(&values, 0.0, 0.0, -1.0).unwrap(), "timed-out");
    assert_eq!(
        atomics::store(&values, f64::NAN, 4_294_967_297.0).unwrap(),
        4_294_967_297.0
    );
    assert_eq!(atomics::load(&values, 0.0).unwrap(), 1.0);
    let view = DataView::from_buffer(buffer.clone()).unwrap();
    assert_eq!(view.get_int32(4.0, true).unwrap(), 1.0);
    let copy = buffer.slice_all();
    assert!(copy.shared_storage().is_some());
    assert_ne!(buffer, copy);
    values.set_number(0.0, 9.0);
    assert_eq!(
        Int32Array::from_buffer_only(copy).unwrap().get_number(1.0),
        Some(1.0)
    );
    let started = Instant::now();
    assert_eq!(atomics::wait(&values, 0.0, 9.0, 20.0).unwrap(), "timed-out");
    assert!(started.elapsed() >= Duration::from_millis(20));
    for index in [-1.0, 2.0, f64::INFINITY] {
        assert!(atomics::wait(&values, index, 9.0, 0.0).is_err());
    }
    let ordinary = Int32Array::new(1.0).unwrap();
    assert!(atomics::wait(&ordinary, 0.0, 0.0, 0.0).is_err());
    assert_eq!(atomics::notify_all(&ordinary, 0.0).unwrap(), 0.0);
    assert_eq!(atomics::store(&ordinary, 0.0, -3.0).unwrap(), -3.0);
    assert_eq!(atomics::load(&ordinary, 0.0).unwrap(), -3.0);
    for invalid in [f64::NAN, -0.5, 0.5, f64::INFINITY, (usize::MAX as u128 + 1) as f64] {
        assert!(ArrayBuffer::new_shared(invalid).is_err());
        assert!(ArrayBuffer::new(invalid).is_err());
    }
    assert_eq!(
        atomics::wait(&values, 0.0, 0.0, f64::MAX).unwrap(),
        "not-equal"
    );
    assert_eq!(buffer, buffer.clone());
    let second = ArrayBuffer::from_shared_storage(buffer.shared_storage().unwrap());
    assert_ne!(buffer, second);
    assert_eq!(
        Int32Array::from_buffer_only(second)
            .unwrap()
            .get_number(1.0),
        Some(9.0)
    );
}

#[test]
fn atomic_notification_selects_exact_address_and_waiter_count() {
    let buffer = ArrayBuffer::new_shared(8.0).unwrap();
    let storage = buffer.shared_storage().unwrap();
    let values = Int32Array::from_buffer_only(buffer).unwrap();
    let (completed, observed) = mpsc::channel();
    let workers: Vec<_> = (0..3)
        .map(|worker| {
            let storage = Arc::clone(&storage);
            let completed = completed.clone();
            std::thread::spawn(move || {
                let values =
                    Int32Array::from_buffer_only(ArrayBuffer::from_shared_storage(storage))
                        .unwrap();
                let offset = if worker == 2 { 1.0 } else { 0.0 };
                let result = atomics::wait(&values, offset, 0.0, 5000.0).unwrap();
                completed.send((worker, result)).unwrap();
            })
        })
        .collect();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        assert_eq!(atomics::notify(&values, 0.0, 0.0).unwrap(), 0.0);
        if atomics::notify(&values, 0.0, 1.0).unwrap() == 1.0 {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    let first = observed.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(first.0 < 2);
    assert_eq!(first.1, "ok");
    assert!(observed.recv_timeout(Duration::from_millis(20)).is_err());
    let mut notified = 0.0;
    while notified < 2.0 {
        notified += atomics::notify_all(&values, 0.0).unwrap();
        notified += atomics::notify_all(&values, 1.0).unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    for _ in 0..2 {
        assert_eq!(
            observed.recv_timeout(Duration::from_secs(1)).unwrap().1,
            "ok"
        );
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(atomics::notify_all(&values, 0.0).unwrap(), 0.0);
}
