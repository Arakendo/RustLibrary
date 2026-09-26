use rustlibrary_ulid::{GenerateError as E, Generator, MAX_TIMESTAMP_MS, ParseError, Ulid};

#[test]
fn canonical_vectors_and_binary_layout() {
    for (bytes, text) in [
        ([0; 16], "00000000000000000000000000"),
        ([255; 16], "7ZZZZZZZZZZZZZZZZZZZZZZZZZ"),
    ] {
        let id = Ulid::from_bytes(bytes);
        assert_eq!(id.to_string(), text);
        assert_eq!(text.parse::<Ulid>().unwrap().to_bytes(), bytes);
        assert_eq!(text.to_lowercase().parse::<Ulid>().unwrap(), id);
    }
    let id: Ulid = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    assert_eq!(id.timestamp_ms(), 1_469_922_850_259);
    let id = Ulid::from_parts(0x0102_0304_0506, [7; 10]).unwrap();
    assert_eq!(&id.to_bytes()[..6], &[1, 2, 3, 4, 5, 6]);
    assert_eq!(&id.to_bytes()[6..], &[7; 10]);
}

#[test]
fn rejects_malformed_and_overflow() {
    assert_eq!(
        "80000000000000000000000000".parse::<Ulid>(),
        Err(ParseError::Overflow)
    );
    for text in [
        "",
        "0000000000000000000000000",
        "000000000000000000000000000",
        "0000000000000000000000000I",
        "0000000000000000000000000L",
        "0000000000000000000000000O",
        "0000000000000000000000000U",
        "0000000000000000000000000 ",
        "000000000000000000000000é",
    ] {
        assert!(text.parse::<Ulid>().is_err(), "{text}");
    }
    assert_eq!(
        Ulid::from_parts(MAX_TIMESTAMP_MS + 1, [0; 10]),
        Err(E::TimestampOutOfRange)
    );
}

#[test]
fn byte_roundtrips_and_ordering() {
    let mut previous = Ulid::from_bytes([0; 16]);
    for timestamp in 1..1024 {
        let id = Ulid::from_parts(
            timestamp,
            timestamp.to_be_bytes().repeat(2)[..10].try_into().unwrap(),
        )
        .unwrap();
        assert_eq!(id.to_string().parse::<Ulid>().unwrap(), id);
        assert!(previous < id);
        assert!(previous.to_string() < id.to_string());
        previous = id;
    }
}

#[test]
fn monotonic_carry_and_failure_recovery() {
    let mut generator = Generator::new();
    let first = generator
        .generate_with(42, |bytes| {
            bytes[9] = 255;
            Ok(())
        })
        .unwrap();
    let second = generator
        .generate_with(42, |_| panic!("same tick must not request entropy"))
        .unwrap();
    assert!(second > first);
    assert_eq!(&second.to_bytes()[14..], &[1, 0]);
    assert_eq!(
        generator.generate_with(41, |_| panic!()),
        Err(E::ClockRegression)
    );
    assert_eq!(
        generator.generate_with(MAX_TIMESTAMP_MS + 1, |_| panic!()),
        Err(E::TimestampOutOfRange)
    );
    assert_eq!(
        generator.generate_with(43, |_| Err(E::EntropyUnavailable)),
        Err(E::EntropyUnavailable)
    );
    assert!(generator.generate_with(42, |_| panic!()).unwrap() > second);
    assert!(generator.generate_with(43, |_| Ok(())).unwrap() > second);
}

#[test]
fn random_overflow_never_wraps_or_advances_time() {
    let mut generator = Generator::new();
    generator
        .generate_with(10, |bytes| {
            *bytes = [255; 10];
            Ok(())
        })
        .unwrap();
    for _ in 0..2 {
        assert_eq!(
            generator.generate_with(10, |_| panic!()),
            Err(E::RandomnessExhausted)
        );
    }
    assert_eq!(
        generator
            .generate_with(11, |_| Ok(()))
            .unwrap()
            .timestamp_ms(),
        11
    );
}

#[test]
fn concurrent_shared_generator_has_no_duplicates() {
    let generator = std::sync::Arc::new(std::sync::Mutex::new(Generator::new()));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let generator = generator.clone();
            std::thread::spawn(move || {
                (0..256)
                    .map(|_| {
                        generator
                            .lock()
                            .unwrap()
                            .generate_with(123, |_| Ok(()))
                            .unwrap()
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let ids: std::collections::BTreeSet<_> = threads
        .into_iter()
        .flat_map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(ids.len(), 1024);
}

#[test]
fn os_entropy_smoke() {
    let a = Generator::new().generate().unwrap();
    let b = Generator::new().generate().unwrap();
    assert_ne!(a, b);
    assert_eq!(a.to_string().parse::<Ulid>().unwrap(), a);
}
