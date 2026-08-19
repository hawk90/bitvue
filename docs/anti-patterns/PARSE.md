# Anti-Pattern Catalog — PARSE: 파서와 Bitstream 안전성

이 문서는 더 큰 안티패턴 카탈로그의 한 카테고리입니다 (전체 인덱스는 별도로 작성 중인 `docs/anti-patterns/INDEX.md` 참고). 여기 항목의 상당수는 fuzzing으로만 실질적으로 검증 가능하며, fuzzing 인프라(cargo-fuzz corpus 등)가 구축되면 각 항목의 "탐지 방법"에서 해당 corpus/harness를 직접 링크할 예정입니다.

---

### PARSE-001: unchecked offset + length
**분류**: PARSE · **심각도**: Critical · **탐지**: Static/Runtime

**나쁜 예**:
```rust
fn read_nal_payload(data: &[u8], offset: usize, length: usize) -> &[u8] {
    // offset과 length가 비트스트림에서 읽은 값이면 데이터 크기를 넘을 수 있다
    &data[offset..offset + length]
}
```

**문제**:
- `offset + length`가 `data.len()`을 넘으면 슬라이싱 시점에 즉시 panic한다.
- `offset + length` 자체가 `usize` 덧셈 overflow를 일으킬 수 있다 (release 빌드에서는 wrapping, debug 빌드에서는 panic — 빌드 모드에 따라 동작이 달라짐).
- 호출자가 이 함수를 신뢰 가능한 것으로 착각하고 별도 검증 없이 반복 호출하는 패턴으로 퍼진다.

**발생 조건**:
- OBU/NAL 헤더에서 읽은 `payload_size` 필드가 파일 실제 크기보다 크게 조작된 fuzzed/truncated 파일.
- 컨테이너(MP4/MKV/IVF)가 선언한 sample size와 실제 기록된 바이트 수가 불일치하는 손상 파일.

**권장**:
```rust
fn read_nal_payload(data: &[u8], offset: usize, length: usize) -> Result<&[u8], ParseError> {
    let end = offset
        .checked_add(length)
        .ok_or(ParseError::OffsetOverflow { offset, length })?;
    data.get(offset..end)
        .ok_or(ParseError::OutOfBounds { offset, end, data_len: data.len() })
}
```
- 슬라이싱 전에 항상 `checked_add`로 범위를 계산하고, `[..]` 대신 `.get(..)`로 `Option`을 받아 처리한다.
- 파서 공통 유틸리티(bit-reader/byte-reader)에 이 검증을 한 곳으로 모아, 각 코덱 크레이트가 재구현하지 않게 한다.

**탐지 방법**:
- Static: clippy `indexing_slicing` 계열 lint, `#![deny(clippy::indexing_slicing)]`를 파서 크레이트에 한정 적용.
- Runtime: cargo-fuzz로 offset/length 필드를 랜덤화한 corpus 실행, panic=abort로 크래시 수집.
- Structural: 코드 리뷰에서 `&data[` 패턴을 grep해 전수 점검.

**예외**:
- 이미 상위에서 `offset + length <= data.len()`을 검증한 직후, 같은 함수 스코프 내 재검증은 과잉일 수 있다(단, 그 경우도 `debug_assert!`로 불변식을 문서화하는 것이 좋다).

**Bitvue 판정**: Suspected (부분 확인, 2026-08-01) — 핵심 저수준 유틸리티는 안전한 패턴을 일관되게 사용: `crates/bitvue-core/src/bitreader.rs`의 `BitReader::read_bits()`는 슬라이싱 전에 `remaining_bits()` 검증(라인 173) 후에만 바이트 접근, `crates/bitvue-formats/src/mp4.rs`의 box 순회 루프(라인 379-386)와 sample offset 계산(라인 276-291)은 `checked_add` + `data.len()` 대조 검증을 명시적으로 수행, `crates/bitvue-hevc/src/nal.rs`의 `find_nal_units`/`parse_nal_units`도 `data[start..end]` 슬라이싱 전에 `start+2 > data.len()` 등 경계 가드가 있음. 다만 10개 코덱 크레이트 전반에 흩어진 수백 개의 `data[a..b]` 슬라이싱 지점을 전수 확인하지는 못했고, 확인한 코드들은 전부 "자체 스캔으로 얻은 내부적으로 bound된 위치"에 대한 슬라이싱이라 안전해 보이지만 예외 사례가 남아있을 수 있어 Suspected로 표기.

---

### PARSE-002: 남은 비트 확인 없이 read_bits
**분류**: PARSE · **심각도**: Critical · **탐지**: Static/Runtime

**나쁜 예**:
```rust
struct BitReader<'a> { data: &'a [u8], bit_pos: usize }

impl<'a> BitReader<'a> {
    fn read_bits(&mut self, n: u32) -> u32 {
        let mut value = 0u32;
        for _ in 0..n {
            let byte = self.data[self.bit_pos / 8]; // 남은 비트 확인 없음
            let bit = (byte >> (7 - self.bit_pos % 8)) & 1;
            value = (value << 1) | bit as u32;
            self.bit_pos += 1;
        }
        value
    }
}
```

**문제**:
- `n`이 남은 비트 수보다 크면 `self.data[..]` 인덱싱이 범위를 벗어나 panic한다.
- 매 비트마다 인덱싱 연산을 하므로 성능도 나쁘고, 안전성 검증 지점이 루프 내부에 흩어져 있어 리뷰하기 어렵다.
- 호출부마다 "이 필드는 몇 비트 남았을 때만 안전한지"를 암묵적으로 가정하게 되어 스펙 변경 시 깨지기 쉽다.

**발생 조건**:
- `ue(v)`/`se(v)` 같은 가변 길이 필드를 파싱한 뒤, 남은 비트가 예상보다 적은 상태에서 다음 고정 길이 필드를 읽을 때.
- 트렁케이트된(파일이 중간에 잘린) 스트림에서 마지막 NAL/OBU를 파싱할 때.

**권장**:
```rust
impl<'a> BitReader<'a> {
    fn read_bits(&mut self, n: u32) -> Result<u32, ParseError> {
        if self.bits_remaining() < n as usize {
            return Err(ParseError::UnexpectedEof { needed: n, remaining: self.bits_remaining() });
        }
        let mut value = 0u32;
        for _ in 0..n {
            let byte = self.data[self.bit_pos / 8];
            let bit = (byte >> (7 - self.bit_pos % 8)) & 1;
            value = (value << 1) | bit as u32;
            self.bit_pos += 1;
        }
        Ok(value)
    }

    fn bits_remaining(&self) -> usize {
        self.data.len() * 8 - self.bit_pos
    }
}
```
- 모든 `read_*` 계열 메서드는 `Result`를 반환하고, 진입부에서 필요한 비트 수를 한 번에 검증한다.
- 파서 상위 레벨에서 `?`로 전파해 malformed 입력을 정상적인 에러 경로로 처리한다.

**탐지 방법**:
- Static: `BitReader`류 타입의 public 메서드가 `Result`/`Option`을 반환하지 않으면 리뷰에서 반려하는 체크리스트 규칙.
- Runtime: 각 코덱 크레이트별 fuzz target에서 잘린 입력(임의 바이트에서 truncate)을 우선순위 corpus로 유지.
- Manual: 코드 리뷰 시 "이 read 함수가 실패할 수 있는가?"를 항상 질문.

**예외**:
- 이미 길이가 고정된 헤더(예: NAL 헤더 첫 2바이트처럼 최소 크기가 상위에서 보장된 영역)를 읽는 경우, 상위에서 최소 길이를 이미 검증했다면 내부에서 `debug_assert`만으로 충분할 수 있다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-core/src/bitreader.rs`의 `BitReader::read_bits()`(라인 160-216)는 실제 비트 읽기 전에 `if self.remaining_bits() < bits_needed { return Err(BitvueError::UnexpectedEof(...)) }`(라인 172-175)로 남은 비트를 먼저 검증하고 `Result`를 반환. `read_bit()`(라인 129)도 동일하게 EOF에서 `Err`를 반환. 이 구현은 HEVC/AVC/VVC/VP9/MPEG2/AV3/AV1 코덱 크레이트가 얇은 wrapper(`self.inner.read_bits(...)`)로 공유하므로, 이 항목의 "나쁜 예"(남은 비트 검증 없이 인덱싱) 패턴은 존재하지 않음. AVS3만 별도 구현(`crates/bitvue-avs3/src/bitreader.rs`)을 갖지만 거기서도 `bits_remaining() < n` 검증 후 `Err` 반환(라인 37-39)으로 동일 원칙을 따름.

---

### PARSE-003: shift width가 타입 크기 이상
**분류**: PARSE · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
fn read_bits_u32(reader: &mut BitReader, n: u32) -> u32 {
    // n이 32 이상이면 "attempt to shift left with overflow"로 panic (debug) 혹은 UB급 wrapping(release)
    let mask: u32 = (1u32 << n) - 1;
    reader.read_raw(n) & mask
}
```

**문제**:
- `n >= 32`일 때 `1u32 << n`은 debug 빌드에서 panic, release 빌드에서는 `1 << (n % 32)`로 잘못된 값을 조용히 반환한다(정의된 동작이지만 의도와 다름).
- 코덱 스펙 상 필드 폭이 32비트를 초과하는 경우(예: 64비트 타임스탬프, extended precision 필드)를 32비트 헬퍼로 잘못 재사용하면 항상 이 문제가 발생한다.
- 릴리즈와 디버그 빌드에서 동작이 달라지므로, 디버그 빌드에서만 잡히고 릴리즈에서는 조용히 틀린 값이 퍼진다.

**발생 조건**:
- 필드 폭을 나타내는 값(`n`)이 상수가 아니라 비트스트림에서 읽은 값(예: HEVC `descriptor` 길이, VVC `general_constraint_info` 가변 폭)일 때 특히 위험.
- `u32`용 헬퍼를 `u64` 필드(PTS/DTS, 큰 offset)에 재사용할 때.

**권장**:
```rust
fn read_bits_u32(reader: &mut BitReader, n: u32) -> Result<u32, ParseError> {
    if n > 32 {
        return Err(ParseError::FieldWidthTooLarge { n, max: 32 });
    }
    let raw = reader.read_raw(n)?;
    let mask: u32 = if n == 32 { u32::MAX } else { (1u32 << n) - 1 };
    Ok(raw & mask)
}
```
- shift 전에 폭 상한을 명시적으로 검증하고, `n == bit_width`인 경계값(전체 마스크)을 별도로 처리한다.
- 가능하면 `checked_shl`/`wrapping_shl` 등 명시적 의미의 API를 사용해 "어떤 동작을 원하는지"를 코드에 남긴다.

**탐지 방법**:
- Static: `#![warn(arithmetic_overflow)]`, clippy `unnecessary_cast`/`cast_possible_truncation`와 함께 shift 관련 lint 활성화.
- Runtime: 필드 폭 자체를 fuzz 입력으로 삼는 조합 테스트(경계값 0, 31, 32, 33, 63, 64).
- Structural: `<<`, `>>` 연산자를 grep해 리터럴이 아닌 shift width를 쓰는 곳을 전수 조사.

**예외**:
- shift width가 컴파일타임 상수이고 타입 크기 이하임이 자명한 경우(`x << 4` on `u32`)는 문제 없음.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-core/src/bitreader.rs`의 `read_bits()`/`read_bits_u64()`가 shift 연산 전에 `if n > 32 { return Err(...) }`(라인 164-169) / `if n > 64 { return Err(...) }`(라인 236-241)로 폭 상한을 명시적으로 검증한 뒤에만 `(1u32 << bits_to_read) - 1`류 마스크 연산을 수행(라인 195-199, 266-270)하며, `bits_to_read == 8`(또는 64) 경계값도 별도 분기로 처리해 이 문서의 "권장" 코드와 동일한 형태. AVS3 독립 구현(`crates/bitvue-avs3/src/bitreader.rs`)도 `debug_assert!(n <= 32)` + 비트 단위 루프라 shift overflow 자체가 발생하지 않는 구조.

---

### PARSE-004: LEB128 최대 길이 제한 부재
**분류**: PARSE · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
// AV1 OBU 헤더의 leb128() 파싱
fn read_leb128(reader: &mut ByteReader) -> u64 {
    let mut value: u64 = 0;
    let mut i = 0;
    loop {
        let byte = reader.read_u8();
        value |= ((byte & 0x7f) as u64) << (i * 7);
        if byte & 0x80 == 0 { break; }
        i += 1; // 종료 바이트가 안 나오면 무한정 증가
    }
    value
}
```

**문제**:
- AV1 스펙은 `leb128`을 최대 8바이트로 제한하지만, 이 구현은 continuation bit(0x80)가 계속 서있는 malformed 입력에서 `i`가 계속 증가한다.
- `i * 7 >= 64`가 되면 `<< (i*7)`가 shift overflow를 일으켜 PARSE-003과 동일한 문제로 이어진다.
- `reader.read_u8()`가 EOF에서 별도 처리 없이 panic하거나 0을 반환하면(PARSE-026 참고) 무한 루프로 발전할 수도 있다.

**발생 조건**:
- OBU 헤더의 `obu_size` 필드가 fuzzing으로 continuation bit이 연속으로 켜진 바이트열로 조작된 경우.
- 유사하게 다른 컨테이너의 vint/varint 인코딩(MKV EBML의 가변 길이 정수 등)에서도 동일 패턴이 반복된다.

**권장**:
```rust
const LEB128_MAX_BYTES: usize = 8; // AV1 spec: leb128 is at most 8 bytes

fn read_leb128(reader: &mut ByteReader) -> Result<u64, ParseError> {
    let mut value: u64 = 0;
    for i in 0..LEB128_MAX_BYTES {
        let byte = reader.read_u8()?;
        let chunk = (byte & 0x7f) as u64;
        value |= chunk
            .checked_shl((i * 7) as u32)
            .ok_or(ParseError::Leb128Overflow)?;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(ParseError::Leb128TooLong)
}
```
- 스펙이 정한 최대 바이트 수를 상수로 박아 루프 상한으로 사용한다(무제한 루프 금지).
- 마지막 유효 바이트에서 상위 비트가 값 표현 범위(`u64`)를 넘는지도 검증하면 더 안전하다(over-long encoding 방지).

**탐지 방법**:
- Static: 가변 길이 정수 디코더 함수에 대해 "루프에 명시적 상한이 있는가"를 리뷰 체크리스트로 강제.
- Runtime: continuation bit이 8개 이상 연속인 입력을 fuzz corpus에 시드로 추가.
- Semantic: over-long encoding(불필요하게 긴 표현)이 스펙 위반임을 검증하는 semantic 테스트.

**예외**:
- 표준이 정한 varint가 원래 무제한 길이를 허용하는 경우는 없지만, 만약 그런 포맷이라면 최소한 "파서가 허용할 실용적 상한"을 별도로 정책화해야 한다(무제한은 곧 DoS 벡터).

**Bitvue 판정**: N/A (확인, 2026-07-31) — `crates/bitvue-av1-codec/src/leb128.rs`가 `decode_uleb128()` 구현: `MAX_LEB128_BYTES: usize = 8` 상수로 `.iter().take(MAX_LEB128_BYTES)`를 사용해 루프 자체가 상한을 가짐(무제한 루프 불가), 8바이트를 다 읽고도 continuation bit이 서 있으면 `"LEB128 exceeded maximum 8 bytes"` 에러 반환. Shift overflow도 `shift >= MAX_LEB128_BITS || (shift > 0 && data_bits > (u64::MAX >> shift))` 체크로 별도 방지 — 나쁜 예의 무한루프/shift-overflow 패턴과 반대로 정확히 이 문서의 "권장" 코드와 같은 형태로 이미 구현되어 있음. 다른 코덱 크레이트(HEVC/AVC의 Exp-Golomb, VP9 등 자체 varint 형식이 있다면)는 미확인 — leb128은 AV1 전용이라 이 판정은 AV1 크레이트에 한정.

---

### PARSE-005: Exp-Golomb prefix 무한 탐색
**분류**: PARSE · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
// HEVC/AVC ue(v) 파싱
fn read_ue(reader: &mut BitReader) -> u32 {
    let mut leading_zero_bits = 0;
    while reader.read_bit() == 0 { // EOF에서도 계속 0을 반환하면 무한 루프
        leading_zero_bits += 1;
    }
    let suffix = reader.read_bits(leading_zero_bits);
    (1 << leading_zero_bits) - 1 + suffix
}
```

**문제**:
- `read_bit()`이 EOF 도달 시 예외 없이 계속 `0`을 반환하는 구현이면(PARSE-026), 이 루프는 실제 입력이 아니라 "패딩된 0"을 무한히 읽어 종료되지 않는다.
- 설령 EOF에서 panic한다 해도, `leading_zero_bits`에 상한이 없으므로 malformed 입력이 32비트를 초과하는 leading zero를 갖도록 조작하면 `1 << leading_zero_bits`가 overflow한다.
- CPU를 점유하는 무한/준무한 루프는 크래시보다 탐지하기 어려운 DoS 벡터다(fuzzer가 타임아웃으로만 겨우 잡아낸다).

**발생 조건**:
- SEI, VUI 등 "건너뛰어도 되는" 선택적 필드를 파싱하다가 값이 훼손된 스트림에서 흔히 발생.
- 네트워크 스트리밍 등에서 패킷이 잘려 뒷부분이 0-padding으로 채워진 버퍼를 그대로 파싱할 때.

**권장**:
```rust
const MAX_UE_LEADING_ZEROS: u32 = 32; // u32 표현 범위를 넘는 leading zero는 malformed로 취급

fn read_ue(reader: &mut BitReader) -> Result<u32, ParseError> {
    let mut leading_zero_bits = 0u32;
    while reader.read_bit()? == 0 {
        leading_zero_bits += 1;
        if leading_zero_bits > MAX_UE_LEADING_ZEROS {
            return Err(ParseError::ExpGolombPrefixTooLong { leading_zero_bits });
        }
    }
    let suffix = reader.read_bits(leading_zero_bits)?;
    Ok((1u32.checked_shl(leading_zero_bits).ok_or(ParseError::Overflow)? - 1)
        .checked_add(suffix)
        .ok_or(ParseError::Overflow)?)
}
```
- `read_bit()` 자체가 EOF에서 명시적으로 에러를 반환하게 만들어(PARSE-026과 연동) 무한 루프의 근본 원인을 차단한다.
- prefix 길이에 상한을 두어, 그 자체로 malformed 스트림을 조기에 거부한다.

**탐지 방법**:
- Static: `while ... == 0 { }` 형태로 상한 없이 도는 루프를 grep/clippy 커스텀 lint로 탐지.
- Runtime: 0xFF...(0-run이 긴) 입력, 그리고 EOF 직전에서 끝나는 입력을 fuzz corpus에 포함.
- Manual: 루프 타임아웃을 건 fuzz harness(예: `-timeout=1`)로 "느려지는" 입력도 크래시로 취급.

**예외**:
- 없음 — Exp-Golomb 계열 디코더는 항상 prefix 상한과 EOF 처리를 함께 가져야 한다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-core/src/bitreader.rs`의 `ExpGolombReader::read_ue()`가 HEVC(`bitvue-hevc`)와 AVC(`bitvue-avc`) 둘 다 공유하는 구현. `MAX_EXP_GOLOMB_ZEROS: u32 = 31`(H.264 스펙상 2^32-1의 최대 leading zero 수) 상한이 두 경로 모두에 적용됨: (1) 32비트 이상 남았을 때의 fast path는 처리 전에 `leading_zeros > MAX_EXP_GOLOMB_ZEROS` 체크("Check BEFORE any processing to prevent bypass" 주석 있음 — 과거에 우회 취약점을 의식적으로 막은 흔적으로 보임), (2) bit-by-bit fallback도 `while leading_zeros <= MAX_EXP_GOLOMB_ZEROS` 루프 상한 + 종료 후 재검증. 나쁜 예의 무한 루프 패턴과 반대로 이미 하드닝되어 있음. 공유 구현이라 CODEC-010(코덱별 BitReader 전체 복제) 안티패턴도 동시에 피해가는 좋은 사례.

---

### PARSE-006: malformed input에서 panic
**분류**: PARSE · **심각도**: Critical · **탐지**: Static/Structural

**나쁜 예**:
```rust
fn parse_sps(data: &[u8]) -> SpsData {
    let profile_idc = data[1]; // 길이 검증 없이 인덱싱
    let level_idc = data[3];
    let sps_id = read_ue(&mut BitReader::new(&data[4..])).unwrap(); // unwrap
    SpsData { profile_idc, level_idc, sps_id }
}
```

**문제**:
- 이 함수 어디서든 실패하면 그대로 `panic!`으로 이어지고, GUI 애플리케이션(Tauri)에서는 프로세스 전체가 죽거나 최소한 해당 분석 세션이 복구 불가능해진다.
- `unwrap()`/`expect()`/인덱싱 panic이 라이브러리 API 경계 밖으로 노출되면, 호출자가 "파일이 이상하다"는 정상적 신호를 받을 방법이 없다.
- 하나의 malformed 파일이 전체 애플리케이션을 다운시킬 수 있다는 것은 분석 도구로서 치명적 — 사용자가 정확히 조사하고 싶은 "이상한 파일"이 도구를 죽이는 역설이 발생한다.

**발생 조건**:
- 사용자가 손상되었거나 다른 코덱으로 오인식된 파일을 드래그 앤 드롭했을 때.
- 개발 중 새 코덱 신택스 요소를 추가하면서 기존 필드 오프셋 가정이 깨졌을 때.
- Fuzzing/adversarial 입력 — 신뢰할 수 없는 소스에서 받은 파일을 분석할 때(가장 중요한 위협 모델).

**권장**:
```rust
fn parse_sps(data: &[u8]) -> Result<SpsData, ParseError> {
    let mut reader = BitReader::new(data);
    let profile_idc = reader.read_u8()?;
    reader.skip_bits(16)?; // constraint flags 등
    let level_idc = reader.read_u8()?;
    let sps_id = reader.read_ue()?;
    Ok(SpsData { profile_idc, level_idc, sps_id })
}
```
- 파서 크레이트 전역에서 `unwrap`/`expect`/`panic!`/직접 인덱싱을 금지하고, 모든 실패 경로가 `Result<_, ParseError>`로 상위까지 전파되게 한다.
- 최상위(Tauri command 경계)에서 `Result::Err`를 catch해 사용자에게 "이 파일은 파싱할 수 없습니다: {reason}"으로 보여준다 — 프로세스는 절대 죽지 않는다.
- `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]`를 파서 크레이트 lint 설정에 건다.

**탐지 방법**:
- Static: clippy `unwrap_used`/`expect_used`/`panic`/`indexing_slicing`을 파서 크레이트 CI 게이트로 강제.
- Runtime: cargo-fuzz로 각 코덱 파서 진입점(`parse_sps`, `parse_obu`, `parse_frame_header` 등)을 대상으로 24시간 이상 연속 fuzzing.
- Structural: `catch_unwind`로 감싸 배포하는 것은 임시방편일 뿐 — panic 자체를 없애는 것이 목표임을 리뷰에서 확인.

**예외**:
- 애플리케이션 시작 시 정적 설정(예: 하드코딩된 상수 테이블)에 대한 `unwrap()`처럼, 입력 데이터와 무관하게 항상 성립하는 불변식에는 예외를 둘 수 있다(다만 `expect("이유")`로 근거를 남길 것).

**Bitvue 판정**: Suspected (부분 확인, 2026-07-31) — `bitvue-av1-codec` 크레이트 직접 감사(가장 위험도 높아 보이던 크레이트, 전체 10개 코덱 크레이트 중 1개만 확인). 단순 `grep '.unwrap()'` 원시 카운트는 195건으로 위협적이었으나, 실제로는 대부분 `tests.rs`(전용 테스트 파일, `#[cfg(test)]` 파일 내부 마커 없이도 테스트 전용)와 doc-comment 예제였음 — grep만으로는 과대평가된다는 걸 이 크레이트에서 직접 확인. 프로덕션 코드에 남는 실제 후보 4건을 전부 문맥 확인:
  - `symbol/cdf.rs:238,280`의 `panic!()` 2건 — 하드코딩 상수 배열(`mv_joint_counts` 등)의 자기 검증용, 입력 데이터로 도달 불가 → N/A
  - `symbol/arithmetic.rs:193`의 `cdf.last().unwrap()` — 바로 위 `if cdf.len() < 2 { return Err(...) }` 가드로 인해 panic 불가능 → 안전
  - `tile/tile_group.rs:167`의 `tile_sb_dimensions(0, 0).unwrap()` — 처음엔 malformed bitstream으로 트리거 가능해 보였으나, `tile_count() == 1`이면 `tile_cols.saturating_mul(tile_rows) == 1`이 성립하고 이는 `tile_cols == 1 && tile_rows == 1`을 강제하므로(둘 다 u32, 곱이 1이려면 둘 다 1) `tile_sb_dimensions(0,0)`의 bounds check(`tile_col >= tile_cols`)를 항상 통과 → 안전
  - **나머지 9개 코덱 크레이트(HEVC 33/AVC 26/VP9 29/VVC 32 등 원시 unwrap 카운트) 미확인** — av1-codec 결과가 좋다고 다른 크레이트도 그렇다고 가정하면 안 됨. Suspected로 남기는 이유.

---

### PARSE-007: slice indexing 직접 사용
**분류**: PARSE · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
fn parse_scaling_list(data: &[u8], offset: usize) -> [u8; 16] {
    let mut list = [0u8; 16];
    for i in 0..16 {
        list[i] = data[offset + i]; // offset이 신뢰되지 않은 값이면 OOB panic
    }
    list
}
```

**문제**:
- PARSE-001과 근본 원인은 같지만, 이 항목은 "습관적으로 `[]`를 쓰는 코딩 스타일" 자체를 별도 문제로 지적한다 — 파서 크레이트 전반에 `[]` 인덱싱이 퍼지면 개별 검증만으로는 전수 방어가 불가능하다.
- 루프 내 인덱싱은 리뷰어가 "이번 접근은 안전한가"를 매번 재계산해야 해서 실수가 누적되기 쉽다.
- `data[offset + i]`처럼 산술식이 인덱스에 바로 들어가면 overflow 위험(PARSE-001)까지 겹친다.

**발생 조건**:
- 코덱 크레이트마다 bit-reader가 별도로 구현되어 있어(각 코덱이 독립 크레이트), 안전한 접근 패턴이 통일되지 않고 크레이트별로 재발한다.
- 리뷰에서 놓치기 쉬운 "테스트 코드에서는 안전하던 헬퍼를 프로덕션 파싱 경로에 그대로 재사용"하는 경우.

**권장**:
```rust
fn parse_scaling_list(data: &[u8], offset: usize) -> Result<[u8; 16], ParseError> {
    let slice = data
        .get(offset..offset + 16)
        .ok_or(ParseError::OutOfBounds { offset, end: offset + 16, data_len: data.len() })?;
    let mut list = [0u8; 16];
    list.copy_from_slice(slice);
    Ok(list)
}
```
- `slice.get(range)` + `copy_from_slice`처럼 "한 번에 범위를 검증하고 복사"하는 패턴으로 통일한다.
- 프로젝트 공통 `SafeReader` 트레이트/타입을 만들어 모든 코덱 크레이트가 동일한 안전 원시 연산 위에서 동작하게 한다(각 크레이트가 bit-reader를 독립적으로 구현하더라도, 최소한 저수준 바이트 접근 유틸은 공유).

**탐지 방법**:
- Static: `#![deny(clippy::indexing_slicing)]`를 파서 크레이트 워크스페이스 lint로 전역 적용 후, 예외가 필요한 곳만 `#[allow]`로 명시.
- Structural: `grep -rn '\[.*\]' --include=*.rs`로 배열/슬라이스 인덱싱 패턴을 주기적으로 스캔해 신규 발생을 추적.

**예외**:
- 이미 `.get()`으로 범위를 검증한 직후, 같은 스코프에서 검증된 서브슬라이스에 대한 고정 상수 인덱싱(`slice[0]`, 길이가 이미 보장된 경우)은 허용 가능.

**Bitvue 판정**: Suspected (부분 확인, 2026-08-01) — PARSE-001과 근본 원인 동일. 확인한 핵심 경로(bitreader.rs, mp4.rs box 파싱, hevc/avc/vvc nal.rs의 NAL 분리)는 `checked_add`/사전 경계 검증 후 슬라이싱하는 패턴을 일관되게 사용하지만, 10개 코덱 크레이트에 각각 독립적으로 존재하는 파서 코드 전체에서 습관적 `[]` 인덱싱 스타일 자체를 전수 조사하지는 못함. `#![deny(clippy::indexing_slicing)]` 같은 워크스페이스 전역 lint가 Cargo.toml에 걸려있는지는 미확인.

---

### PARSE-008: 파일 offset을 usize로 표현
**분류**: PARSE · **심각도**: Medium · **탐지**: Static/Structural

**나쁜 예**:
```rust
struct SampleEntry {
    offset: usize, // 32비트 타겟(예: wasm32)에서 4GB 초과 파일의 offset을 표현 불가
    size: usize,
}

fn read_sample(file: &mut File, entry: &SampleEntry) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(entry.offset as u64))?; // as u64는 안전하지만 저장 시점에 이미 손실
    ...
}
```

**문제**:
- `usize`는 플랫폼 의존 크기(32비트 타겟에서는 4바이트)라서, 4GB를 초과하는 대용량 영상 파일(특히 고해상도/장시간 캡처)의 offset을 표현하지 못한다.
- Tauri 앱은 데스크톱 타겟이 주력이지만, 향후 wasm/embedded 타겟이나 32비트 빌드를 고려하면 이 가정이 조용히 깨질 수 있다.
- 컨테이너 포맷(MP4의 `co64`, MKV의 큰 Cluster 등)은 애초에 64비트 offset을 염두에 두고 설계되어 있어, 파서가 이를 `usize`로 강제로 축소하면 스펙과 어긋난다.

**발생 조건**:
- 대용량 4K/8K RAW 소스나 긴 녹화본(수십 GB) 파일을 여러 시간 분석할 때.
- 32비트 빌드 타겟이나 향후 브라우저(WASM) 포팅을 고려하는 시점.

**권장**:
```rust
struct SampleEntry {
    offset: u64, // 컨테이너 스펙의 offset 필드 폭에 맞춰 명시적 고정 크기 사용
    size: u64,
}

fn read_sample(file: &mut File, entry: &SampleEntry) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(entry.offset))?;
    let size = usize::try_from(entry.size)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "sample size exceeds usize"))?;
    let mut buf = vec![0u8; size];
    file.read_exact(&mut buf)?;
    Ok(buf)
}
```
- 파일/스트림 offset과 size는 항상 `u64`로 저장하고, 실제 메모리 슬라이스에 접근하는 마지막 순간에만 `usize`로 `try_from` 변환(실패 시 에러)한다.
- in-memory 버퍼 인덱스(이미 메모리에 있는 `&[u8]`에 대한 오프셋)는 `usize`가 맞지만, "파일 상의 절대 위치"와 "메모리 버퍼 내 상대 위치"를 타입 수준에서 구분하면 혼동을 막을 수 있다(newtype 권장).

**탐지 방법**:
- Static: 파일 offset/size를 다루는 구조체 필드에 `usize` 사용을 금지하는 리뷰 체크리스트, 또는 newtype(`FileOffset(u64)`) 강제.
- Structural: `as usize` 캐스팅 지점을 grep해 offset/size 관련 캐스팅만 별도로 감사.

**예외**:
- 이미 메모리에 로드된 `&[u8]` 버퍼 내부의 상대 offset(파일 전체가 아니라 이미 읽어들인 슬라이스 기준)은 `usize`가 자연스럽고 올바르다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-formats/src/mp4.rs`의 `BoxHeader.size`/`data_offset`, `Mp4Info.sample_offsets` 등 파일 오프셋 관련 필드는 전부 `u64`로 저장되고, 실제 메모리 슬라이스 접근 직전에만 `usize::try_from(**offset_ptr).map_err(...)`(라인 276-282)로 명시적 실패 처리하며 변환. `crates/bitvue-av1-codec/src/ivf.rs`도 `offset: usize`를 쓰지만 이는 이미 전체가 메모리에 로드된 `&[u8]` 버퍼 내 상대 오프셋이라 이 항목의 "예외" 조항에 해당.

---

### PARSE-009: byte offset과 bit offset 혼용
**분류**: PARSE · **심각도**: High · **탐지**: Static/Structural

**나쁜 예**:
```rust
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize, // 이 pos가 byte 단위인지 bit 단위인지 타입에 드러나지 않는다
}

fn skip_to_byte_boundary(reader: &mut BitReader) {
    reader.pos = (reader.pos + 7) / 8; // 호출부에 따라 버그: pos가 이미 byte 단위면 완전히 틀린 결과
}
```

**문제**:
- `pos: usize` 하나로 비트 위치와 바이트 위치를 겸용하면, 함수마다 "이 필드가 지금 무슨 단위인가"를 문서나 이름에 의존해 추측해야 한다.
- 코드가 커지면서 어떤 헬퍼는 비트 단위로, 어떤 헬퍼는 바이트 단위로 같은 필드를 다뤄 조용히 8배 어긋난 위치를 읽는 버그가 생긴다(컴파일러가 잡아주지 못함).
- 여러 코덱 크레이트가 독립적으로 bit-reader를 구현하는 이 프로젝트 구조상, 크레이트마다 관례가 달라(어떤 크레이트는 `bit_pos`, 어떤 크레이트는 `byte_pos + bit_offset` 분리) 크레이트를 넘나들며 작업할 때 특히 위험하다.

**발생 조건**:
- RBSP 파싱(비트 단위)과 EBSP 스캔(바이트 단위, emulation prevention byte 탐색)을 같은 리더 안에서 오가는 코드.
- 컨테이너 파서(바이트 단위)가 만든 offset을 코덱 파서(비트 단위)에 그대로 넘기는 경계 지점.

**권장**:
```rust
struct BitPos(u64); // newtype으로 단위를 타입에 고정
struct BytePos(u64);

impl BitPos {
    fn to_byte_floor(&self) -> BytePos { BytePos(self.0 / 8) }
    fn is_byte_aligned(&self) -> bool { self.0 % 8 == 0 }
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: BitPos, // 필드 이름과 타입 모두 단위를 명시
}
```
- byte offset과 bit offset을 서로 다른 newtype으로 분리해, 컴파일러가 단위 혼용을 타입 에러로 잡아주게 한다.
- 두 단위 간 변환은 명시적 메서드(`to_byte_floor`, `to_bit`)로만 허용하고, 암묵적 `as`/산술 변환은 금지한다.
- 여러 코덱 크레이트가 공유하는 `bitstream-io` 유틸 크레이트를 만들어 이 규약을 프로젝트 전역에서 통일하는 것을 고려한다.

**탐지 방법**:
- Static: `pos`, `offset` 등 단위가 불명확한 필드명을 grep해 newtype 적용 여부 감사.
- Structural: 코덱 크레이트 간 bit-reader 구현을 비교해 단위 표현 방식이 일치하는지 아키텍처 리뷰.
- Manual: 컨테이너→코덱 경계에서 offset이 전달되는 모든 지점을 리스트업해 단위 문서화 여부 확인.

**예외**:
- 성능이 극도로 중요한 hot loop 내부에서, 이미 충분히 테스트된 단일 함수 안에서만 원시 정수로 비트/바이트를 다루는 것은 허용 가능(단, 함수 경계를 벗어나지 않아야 함).

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-core/src/bitreader.rs`의 `BitReader` 구조체는 `byte_offset: usize`와 `bit_offset: u8`(0-7, MSB first 명시 주석)를 별도 필드로 분리해 보관(라인 53-60), 완전한 newtype 패턴(`BitPos`/`BytePos`)은 아니지만 필드명과 타입이 명확히 단위를 구분하고, 이 단일 구현을 HEVC/AVC/VVC/VP9/MPEG2/AV3/AV1이 wrapper로 공유하므로 크레이트마다 관례가 갈릴 여지가 구조적으로 차단됨. AVS3 독립 구현도 동일하게 `byte_pos`/`bit_pos`를 분리.

---

### PARSE-010: parser state rollback 불완전
**분류**: PARSE · **심각도**: High · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
fn try_parse_vui(reader: &mut BitReader, sps: &mut Sps) -> Result<(), ParseError> {
    let start = reader.bit_pos();
    sps.vui_present = true; // 실패 가능성이 있는데 미리 상태를 바꿔버림
    let aspect_ratio = reader.read_bits(8)?; // 여기서 실패하면?
    sps.aspect_ratio = Some(aspect_ratio);
    Ok(())
}

fn parse_sps(reader: &mut BitReader) -> Result<Sps, ParseError> {
    let mut sps = Sps::default();
    if reader.read_bit()? == 1 {
        if try_parse_vui(reader, &mut sps).is_err() {
            // reader.bit_pos는 되돌리지 않고 그냥 무시 — sps.vui_present=true인데 필드는 일부만 채워짐
        }
    }
    Ok(sps)
}
```

**문제**:
- `try_parse_vui`가 중간에 실패해도 `sps.vui_present = true`로 이미 바뀐 상태가 그대로 남아, 이후 코드가 "VUI가 있다"고 믿고 `sps.aspect_ratio` 등을 읽으면 잘못된 기본값(`None`)을 유효한 값으로 오해한다.
- `reader`의 비트 위치도 실패 지점에서 멈춘 채로 남아, 호출자가 "실패했으니 이 옵셔널 섹션을 건너뛰고 다음 필드로 가자"고 판단해도 실제로는 어중간한 위치에 있어 이후 파싱이 전부 어긋난다.
- 부분 실패를 "무시하고 계속 진행"하는 패턴은 에러를 삼켜서(swallow) 디버깅을 어렵게 만들고, 손상된 state가 이후 로직에 은밀히 전파된다.

**발생 조건**:
- 스펙상 optional/conditional한 신택스 구조(VUI, SEI, HRD parameters 등)를 파싱하다가 그 내부에서 malformed 데이터를 만났을 때.
- "이 섹션은 파싱 실패해도 전체 파싱은 계속하고 싶다"는 관대한(lenient) 파싱 정책을 구현하려 할 때 흔히 이 실수가 생긴다.

**권장**:
```rust
fn try_parse_vui(reader: &mut BitReader) -> Result<VuiParameters, ParseError> {
    let aspect_ratio = reader.read_bits(8)?;
    Ok(VuiParameters { aspect_ratio })
}

fn parse_sps(reader: &mut BitReader) -> Result<Sps, ParseError> {
    let mut sps = Sps::default();
    if reader.read_bit()? == 1 {
        let checkpoint = reader.bit_pos();
        match try_parse_vui(reader) {
            Ok(vui) => sps.vui = Some(vui),
            Err(e) => {
                reader.seek_bit(checkpoint); // 명시적 rollback
                sps.vui = None;
                sps.warnings.push(ParseWarning::VuiSkipped(e));
            }
        }
    }
    Ok(sps)
}
```
- 옵셔널 서브구조는 별도 함수가 "성공 시에만" 결과 값을 만들어 반환하게 하고(부분 상태를 밖으로 노출하지 않음), 호출부가 `Ok`일 때만 부모 상태에 반영한다.
- 실패 시 reader 위치를 체크포인트로 명시적으로 되돌리고, 무엇이 스킵되었는지 경고로 남긴다(조용히 삼키지 않음).

**탐지 방법**:
- Static: "실패 가능한 호출 이전에 상위 상태(struct field)를 먼저 mutate하는" 패턴을 리뷰에서 우선 점검.
- Runtime: 옵셔널 서브구조 파싱을 의도적으로 실패시키는 fuzz/unit 테스트로, 실패 후 reader 위치와 struct 상태의 일관성을 검증.
- Structural: 파서 함수 시그니처 컨벤션 — "부분 상태를 mutate하는 함수는 실패하지 않는다" 또는 "실패 가능한 함수는 완성된 값만 반환한다"를 아키텍처 규칙으로 문서화.

**예외**:
- 애초에 "부분적으로 파싱된 결과라도 최대한 보존"하는 것이 명시적 요구사항(예: 손상 파일 복구 도구)이라면, `vui_present`와 `vui_partial: PartialVui` 같은 필드를 분리해 "부분 상태"임을 타입으로 명확히 구분해야 한다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-hevc/src/sps.rs`의 VUI 파싱(라인 433-437)은 `vui_parameters = if flag { Some(parse_vui_parameters(&mut reader)?) } else { None }` 형태로, `parse_vui_parameters`가 실패하면 `?`가 `parse_sps` 전체를 즉시 `Err`로 종료시켜 "미리 상태를 바꾼 뒤 부분 실패"가 반환되는 경로 자체가 없음(이 문서의 나쁜 예처럼 `vui_present`를 먼저 true로 설정해두는 패턴이 아님). 다만 이는 "롤백 후 관대하게 계속 진행"이 아니라 "VUI 실패 시 SPS 전체 실패"라는 엄격한 정책이라, PARSE-019가 권장하는 관대한 파싱과는 다른 트레이드오프.

---

### PARSE-011: partial parse 실패 후 state 오염
**분류**: PARSE · **심각도**: High · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
struct DecoderContext {
    active_sps: HashMap<u8, Sps>,
    active_pps: HashMap<u8, Pps>,
}

impl DecoderContext {
    fn process_nal(&mut self, nal: &[u8]) -> Result<(), ParseError> {
        let sps = parse_sps(nal)?; // 파싱 도중 실패하면 그냥 Err 리턴
        self.active_sps.insert(sps.id, sps); // 성공 시에만 여기 도달하므로 이 자체는 안전해 보이지만...
        // 문제: parse_sps 내부에서 이미 self.active_sps를 참조/수정하는 구조라면?
        Ok(())
    }
}
```

**문제**:
- 위 예시처럼 "함수가 성공한 뒤에만 전역 state를 갱신"하는 형태처럼 보여도, 실제 코덱 파서는 파싱 도중 참조 카운트, 캐시, 통계(예: NAL 카운터, 에러 카운터)를 즉시 갱신하는 경우가 많아 실패 시에도 일부 카운터만 증가한 채 남는다.
- 여러 NAL/OBU를 순차 처리하는 루프에서 `i`번째가 실패했을 때, `i-1`번째까지 반영된 state와 `i`번째의 "일부만 반영된" state를 구분하지 못하면 이후 `i+1`번째 파싱이 오염된 컨텍스트 위에서 진행된다.
- 특히 참조 프레임 관리(DPB), 파라미터 셋 캐시처럼 "누적되는 state"는 한 번 오염되면 이후 모든 프레임 파싱 결과가 신뢰할 수 없어진다 — 디버깅 시 원인이 훨씬 이전 프레임이라 추적이 어렵다.

**발생 조건**:
- 스트림 중간에 손상된 NAL/OBU가 하나 섞여 있고, 그 이후의 정상 NAL들도 계속 분석해야 하는 "복원력 있는(resilient) 파싱" 시나리오.
- 파라미터 셋(SPS/PPS/VPS) 갱신이 실패했는데 이전 파라미터 셋이 이미 evict된 경우.

**권장**:
```rust
impl DecoderContext {
    fn process_nal(&mut self, nal: &[u8]) -> Result<(), ParseError> {
        // 1) 순수 함수로 파싱만 수행 — self를 참조하지 않는다
        let sps = parse_sps(nal).map_err(|e| {
            self.stats.parse_errors += 1; // 실패 통계만 별도로, 명시적으로 갱신
            e
        })?;
        // 2) 검증까지 통과한 완성된 값만 커밋
        self.active_sps.insert(sps.id, sps);
        self.stats.sps_parsed += 1;
        Ok(())
    }
}
```
- "파싱(순수 계산)"과 "커밋(state 반영)"을 단계적으로 분리한다 — 파싱 함수는 `&self`/`&mut self`에 의존하지 않는 순수 함수로 만들고, 성공한 완성된 값만 커밋 단계에서 state에 반영한다.
- 실패 시 갱신되는 state(에러 카운터 등)는 "실패를 기록하기 위한 것"임을 명확히 하고, 성공 경로의 state와 분리해 리뷰 시 구분되게 한다.

**탐지 방법**:
- Structural: 파서 함수가 `&mut self`(누적 state를 가진 컨텍스트)를 받는 경우, 함수 내부에서 `?` 이전에 `self`를 mutate하는지 전수 검사.
- Runtime: 정상 NAL 스트림 중간에 하나씩 손상된 NAL을 주입하며 "그 이후 정상 NAL들이 올바르게 파싱되는지" 회귀 테스트.
- Manual: DPB/파라미터셋 캐시처럼 장기 누적되는 state 구조를 문서화하고, 각 갱신 지점이 "완성된 값만 커밋"하는지 리뷰.

**예외**:
- 통계/로깅 목적의 state(에러 카운트 등)는 실패 시에도 갱신되는 것이 오히려 의도된 동작이다 — 다만 "디코딩에 영향을 주는 state"와 "관찰용 state"를 구분해야 한다.

**Bitvue 판정**: Suspected (미확인) — `static mut` 등 전역 mutable 컨텍스트는 없음(PARSE-012 참고)이라 이 항목이 우려하는 최악의 형태(전역 DPB/파라미터셋 캐시 오염)는 구조적으로 발생하기 어려워 보이나, SPS/PPS 파라미터셋 캐시나 다중 NAL 순차 처리 루프에서 "i번째 실패가 i+1번째 파싱 컨텍스트를 오염시키는지"를 직접 재현/코드 추적하지는 못함. 시간 제약으로 깊이 있는 확인을 하지 못해 Suspected로 남김.

---

### PARSE-012: codec state를 전역 mutable 객체로 관리
**분류**: PARSE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
// 코덱 크레이트 전역에 static mutable 컨텍스트
static mut CURRENT_SPS: Option<Sps> = None;

fn parse_slice_header(data: &[u8]) -> Result<SliceHeader, ParseError> {
    unsafe {
        let sps = CURRENT_SPS.as_ref().ok_or(ParseError::MissingSps)?;
        // sps를 참조해 slice header 파싱...
        todo!()
    }
}
```

**문제**:
- `static mut`은 그 자체로 `unsafe`를 요구하며, 멀티스레드 환경(여러 파일을 동시에 분석하거나 프레임을 병렬로 파싱하는 경우)에서 data race의 근원이 된다.
- 전역 상태는 "이 함수가 어떤 입력에 의존하는지"를 시그니처만으로 알 수 없게 만들어 테스트를 어렵게 한다 — 단위 테스트가 전역 상태 초기화 순서에 의존하게 된다.
- 여러 파일/스트림을 동시에 열어 비교하는 것이 핵심 기능인 비트스트림 분석 도구에서, 전역 단일 컨텍스트는 애초에 요구사항과 충돌한다(두 번째 파일을 열면 첫 번째 파일의 SPS가 덮어써짐).

**발생 조건**:
- 여러 영상 파일을 동시에 열어 나란히 비교 분석하는 멀티 문서/멀티 탭 UI에서, 두 번째 파일을 열자 첫 번째 파일의 파라미터 셋이 오염되는 형태로 나타난다.
- 백그라운드에서 썸네일/프리뷰 생성과 메인 분석이 동시에 같은 코덱 파서를 호출할 때 레이스 컨디션으로 나타난다.

**권장**:
```rust
struct DecoderContext {
    active_sps: HashMap<u8, Sps>,
}

impl DecoderContext {
    fn parse_slice_header(&self, data: &[u8], sps_id: u8) -> Result<SliceHeader, ParseError> {
        let sps = self.active_sps.get(&sps_id).ok_or(ParseError::MissingSps { sps_id })?;
        // sps를 명시적 인자로 받아 파싱...
        todo!()
    }
}
```
- 코덱 state는 항상 명시적으로 생성/전달되는 `DecoderContext`(혹은 코덱별 등가 구조체) 인스턴스에 담고, 함수는 이를 매개변수로 받는다.
- 여러 파일을 동시에 열어야 한다면 파일마다 독립된 `DecoderContext` 인스턴스를 만들어, 서로 격리를 자연스럽게 보장한다.
- 병렬 처리(예: rayon으로 프레임 단위 병렬 파싱)가 필요하면 `DecoderContext`를 `Arc<RwLock<_>>`로 감싸거나, 읽기 전용 스냅샷(`Arc<Sps>`)만 공유하는 설계로 전환한다.

**탐지 방법**:
- Static: `grep -rn 'static mut'`로 전역 가변 상태를 전수 조사, `cargo clippy`의 `mutable_key_type`/관련 lint 확인.
- Structural: 코덱 크레이트 공개 API가 `&self`/`&mut self` 없이 순수 전역 함수로만 구성되어 있는지 아키텍처 리뷰.
- Runtime: 두 개 이상의 파일을 동시에 로드하는 통합 테스트로 상태 격리를 검증(loom 등으로 동시성 이슈까지 검증 가능).

**예외**:
- 프로세스 전체에서 단 하나만 존재해야 하는 순수 설정값(예: 로그 레벨)은 `OnceLock`/`lazy_static` 같은 안전한 전역 초기화 패턴으로 허용 가능 — 단, 이는 "파싱 상태"가 아니라 "설정"이어야 한다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `grep -rn "static mut" crates --include="*.rs"` 결과 bitvue 코드 전체에서 `static mut`는 전무하고, 유일한 매치는 `crates/vendor/abseil/src/absl_base/call_once.rs`의 벤더링된 서드파티 코드(주석/테스트 예제)뿐. 코덱 크레이트들은 `DecoderContext`류 구조체나 함수 인자로 상태를 명시적으로 전달하는 구조(예: `ParserFactory::create(CodecType)`, `AV1ParserStrategy` 등)를 사용해 이 항목이 우려하는 전역 mutable state 패턴이 근본적으로 존재하지 않음.

---

### PARSE-013: recursion depth 제한 부재
**분류**: PARSE · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
// VVC/AV1의 중첩 partition tree(quad/multi-type tree split) 파싱
fn parse_coding_tree(reader: &mut BitReader, depth: u32) -> Result<CodingTree, ParseError> {
    let split_flag = reader.read_bit()?;
    if split_flag == 1 {
        let children = (0..4)
            .map(|_| parse_coding_tree(reader, depth + 1)) // depth 상한 없이 재귀
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CodingTree::Split(children))
    } else {
        Ok(CodingTree::Leaf(parse_cu(reader)?))
    }
}
```

**문제**:
- `split_flag`가 조작되어 계속 1로 읽히면, 재귀 호출이 스택 오버플로우를 일으킬 때까지 계속된다 — 스택 오버플로우는 `Result`로 복구할 수 없는 프로세스 크래시다(PARSE-006의 panic보다 더 catch하기 어렵다).
- VVC의 MTT(Multi-Type Tree)처럼 실제 스펙상으로도 여러 단계의 중첩 분할이 가능한 구조는, "스펙이 허용하는 최대 깊이"를 파서가 명시적으로 알고 있어야 하는데 이를 코드에 반영하지 않으면 스펙 밖의 깊이까지 허용해버린다.
- 재귀 파싱은 특히 fuzzer가 찾아내기 쉬운 취약점이다 — 랜덤 비트가 우연히도 "항상 split"을 만들어내는 입력을 금방 생성한다.

**발생 조건**:
- VVC coding quadtree / MTT, AV1 partition tree, HEVC CU quadtree처럼 재귀적으로 정의된 신택스 구조를 파싱할 때.
- 컨테이너 포맷의 중첩 박스 구조(MP4 `moov` 안의 중첩 `trak`/`mdia`/... 박스)를 재귀 파서로 구현할 때도 동일 패턴.

**권장**:
```rust
const MAX_CODING_TREE_DEPTH: u32 = 8; // 스펙이 정의하는 CTU 크기와 최소 CU 크기로부터 유도된 상한

fn parse_coding_tree(reader: &mut BitReader, depth: u32) -> Result<CodingTree, ParseError> {
    if depth > MAX_CODING_TREE_DEPTH {
        return Err(ParseError::RecursionDepthExceeded { depth, max: MAX_CODING_TREE_DEPTH });
    }
    let split_flag = reader.read_bit()?;
    if split_flag == 1 {
        let children = (0..4)
            .map(|_| parse_coding_tree(reader, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CodingTree::Split(children))
    } else {
        Ok(CodingTree::Leaf(parse_cu(reader)?))
    }
}
```
- 스펙에서 유도 가능한 최대 깊이(예: CTU 크기 64에서 최소 CU 크기 4까지 나눌 수 있는 분할 횟수)를 상수로 명시하고, 재귀 진입 시마다 검증한다.
- 가능하다면 재귀 대신 명시적 스택(`Vec`)을 사용한 반복문으로 전환해, 스택 오버플로우 자체를 원천적으로 제거하는 것도 고려한다(깊이가 매우 클 수 있는 컨테이너 박스 파싱에는 특히 권장).

**탐지 방법**:
- Static: 파서 크레이트 내 재귀 함수를 전수 조사(`grep -rn 'fn parse.*depth'` 등)하고 각각 상한 검증 존재 여부 확인.
- Runtime: 재귀 유발 신택스 요소를 "항상 최대로 분할"하도록 강제하는 stress 입력으로 스택 사용량 측정(`RUST_MIN_STACK` 조정 후에도 realistic depth로 실패하는지 확인).
- Structural: 재귀 파서는 반드시 depth 파라미터를 갖고, 그 파라미터가 함수 진입부에서 검증되는지 코드 리뷰 체크리스트화.

**예외**:
- 스펙이 재귀 깊이를 명시적으로 무제한 허용하지 않는 한(그런 경우는 실질적으로 없음) 예외 없음. 반복문으로 변환 가능한 경우 항상 반복문을 우선한다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — 컨테이너 레벨: `crates/bitvue-formats/src/mp4.rs`가 `MAX_BOX_DEPTH: u8 = 16` 상수와 함께 `parse_moov`/`parse_trak`/`parse_mdia`/`parse_minf`/`parse_stbl` 각 함수 진입부에서 "SECURITY: Check box nesting depth to prevent stack overflow" 주석과 함께 `if depth >= MAX_BOX_DEPTH { return Err(...) }`를 명시적으로 검증(라인 452-461 등). 코덱 레벨: `crates/bitvue-av1-codec/src/tile/partition.rs`의 `parse_partition_recursive`가 `MAX_PARTITION_DEPTH: u8 = 10` 상수로 재귀 깊이를 검증(라인 549, 562). HEVC/AVC/VP9/AVS3 크레이트는 실제 CU quadtree/split_cu_flag를 재귀적으로 디코딩하는 파서가 존재하지 않아(overlay_extraction은 단순 반복 루프) 이 항목이 우려하는 무제한 재귀 위험 자체가 구조적으로 부재. VVC도 CTU당 flat loop만 사용하고 진짜 MTT 재귀 파서는 없음.

---

### PARSE-014: 거대한 count를 신뢰해 allocation
**분류**: PARSE · **심각도**: Critical · **탐지**: Static/Runtime

**나쁜 예**:
```rust
fn parse_ref_pic_list(reader: &mut BitReader) -> Result<Vec<RefPic>, ParseError> {
    let num_ref_pics = reader.read_ue()?; // 이론상 32비트까지 표현 가능한 값
    let mut list = Vec::with_capacity(num_ref_pics as usize); // 즉시 거대 할당 시도
    for _ in 0..num_ref_pics {
        list.push(parse_ref_pic(reader)?);
    }
    Ok(list)
}
```

**문제**:
- `num_ref_pics`는 스펙상 실질적으로 수십 개를 넘지 않지만, Exp-Golomb 인코딩은 이론상 4바이트대 값까지 표현 가능하다 — malformed 입력이 `num_ref_pics = 4_000_000_000`을 만들면 `Vec::with_capacity`가 그만큼의 메모리를 즉시 요청한다.
- `with_capacity`는 실패 시 OOM으로 프로세스가 죽거나(allocator가 abort), 시스템 전체가 스와핑으로 멎는 서비스 거부(DoS)로 이어진다 — 단 하나의 malformed 파일로 분석 도구를 마비시킬 수 있다.
- 설령 `for _ in 0..num_ref_pics` 루프가 중간에 `parse_ref_pic`에서 실패해 `?`로 조기 종료되더라도, `with_capacity` 호출 자체는 루프 진입 전에 이미 실행되어 이 시점에 이미 취약하다.

**발생 조건**:
- SPS/PPS의 reference picture set, VVC/HEVC의 `num_short_term_ref_pic_sets`, AV1의 `tile_cols * tile_rows`, 컨테이너의 sample count 등 "개수를 먼저 읽고 그만큼 반복 파싱"하는 모든 구조.
- Fuzzing이나 adversarial 입력이 이 count 필드만 골라 극단값으로 조작하는 것은 매우 흔한 공격 패턴이다(구현이 간단하고 효과가 크기 때문).

**권장**:
```rust
const MAX_REF_PICS: u32 = 16; // 코덱 스펙이 명시하는 실제 상한(예: HEVC는 최대 16)

fn parse_ref_pic_list(reader: &mut BitReader) -> Result<Vec<RefPic>, ParseError> {
    let num_ref_pics = reader.read_ue()?;
    if num_ref_pics > MAX_REF_PICS {
        return Err(ParseError::CountExceedsSpecLimit {
            field: "num_ref_pics",
            value: num_ref_pics,
            max: MAX_REF_PICS,
        });
    }
    let mut list = Vec::with_capacity(num_ref_pics as usize); // 이제 상한이 검증된 뒤이므로 안전
    for _ in 0..num_ref_pics {
        list.push(parse_ref_pic(reader)?);
    }
    Ok(list)
}
```
- 모든 "count 필드"는 파싱 직후, 할당 전에 코덱 스펙이 정의한 상한과 대조해 검증한다(스펙에 상한이 없으면 "실용적 상한" — 예: 남은 바이트 수로 계산 가능한 최대 요소 개수 — 을 정책으로 정한다).
- `with_capacity` 대신 처음에는 작은 용량으로 시작해 실제 파싱이 성공할 때마다 `push`하는(자연 증가) 방식도 극단값 방어에 도움이 된다 — 단, 최선은 상한 검증이지 이것이 대체재는 아니다.
- 이런 상한 검증을 파서 유틸리티 함수(`read_bounded_count`)로 공통화해 모든 코덱 크레이트가 재사용하게 한다.

**탐지 방법**:
- Static: `with_capacity(`, `Vec::from_elem(`, `vec![0; n]` 등 "읽은 값으로 즉시 할당하는" 패턴을 grep해 상한 검증 존재 여부 확인.
- Runtime: count 필드를 `u32::MAX` 근처로 설정한 fuzz corpus, 그리고 메모리 제한을 건 sandbox(`ulimit -v`)에서 fuzzing해 OOM이 크래시로 잡히는지 확인.
- Semantic: 각 count 필드에 대해 스펙 문서에서 상한을 찾아 상수 테이블로 정리(코덱별 문서화 산출물로 남기면 이후 리뷰가 쉬워짐).

**예외**:
- count의 실제 상한이 "남은 파일 크기"로만 자연스럽게 제한되는 가변 개수 구조(예: 개별 원소 최소 크기가 크고 남은 바이트 수로 최대 개수가 이미 작게 제한되는 경우)는, 스펙 상한이 없어도 `remaining_bytes / min_element_size`를 상한으로 사용할 수 있다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-formats/src/mp4.rs`의 `parse_stsz`가 `sample_count > MAX_ENTRY_COUNT`(10,000,000, 라인 793-798)를 할당 전에 검증; `crates/bitvue-av1-codec/src/tile/tile_group.rs`의 `TileInfo::new`가 `MAX_TILE_COLS`/`MAX_TILE_ROWS`/`MAX_TOTAL_TILES` 검증 후에만 구조체 생성(라인 55-79); HEVC/AVC/VVC/VP9/AV1의 `overlay_extraction.rs` 내 `Vec::with_capacity(total_blocks)`류 호출은 전부 SPS의 검증된 pic_width/height(PARSE-015 참고)로부터 `checked_mul`을 거쳐 유도된 값을 사용(예: `crates/bitvue-hevc/src/overlay_extraction.rs:170-174`). `crates/bitvue-av1-codec/src/ivf.rs`도 `IVF_MAX_FRAME_SIZE` 상한을 할당 전 검증. 확인한 범위 내에서 이 항목의 나쁜 예(검증 없이 즉시 `with_capacity`) 패턴을 발견하지 못함.

---

### PARSE-015: width × height overflow
**분류**: PARSE · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
fn compute_frame_buffer_size(width: u32, height: u32, bytes_per_pixel: u32) -> usize {
    (width * height * bytes_per_pixel) as usize // u32 곱셈이 조용히 wrap될 수 있음
}
```

**문제**:
- `width`, `height`가 비트스트림에서 읽은 값(예: HEVC `pic_width_in_luma_samples`)이면 스펙상 상한이 있어도(HEVC는 최대 16888) 파서가 그 상한을 검증하지 않았다면 임의의 32비트 값이 들어올 수 있다.
- `width * height * bytes_per_pixel`은 release 빌드에서 overflow 시 wrapping되어 훨씬 작은 값을 조용히 반환한다 — 이후 이 작은 크기로 버퍼를 할당해놓고, 실제로는 큰 해상도로 픽셀 데이터를 채우려 하면 버퍼 오버런으로 이어진다(메모리 안전성 문제로 직결).
- Debug 빌드에서는 panic하므로, 로컬 개발/테스트에서는 안 보이다가 release 빌드 배포판에서만 조용히 틀린 동작 → 버퍼 오버런으로 나타나는 전형적인 "빌드 모드 의존 버그".

**발생 조건**:
- 해상도 필드가 극단적으로 큰 값(예: `width = 0xFFFF`, `height = 0xFFFF`)으로 조작된 SPS/시퀀스 헤더.
- 8K/16K 등 초고해상도를 실제로 지원하려다 상한 검증을 느슨하게 잡은 경우에도 정상 입력 근처에서 경계값 버그로 나타날 수 있다.

**권장**:
```rust
const MAX_DIMENSION: u32 = 16_384; // 코덱/프로파일이 실제로 허용하는 최대치(스펙 대조 후 결정)
const MAX_FRAME_BUFFER_BYTES: usize = 2_usize.pow(30); // 실용적 상한(1GiB 등, 정책으로 결정)

fn compute_frame_buffer_size(width: u32, height: u32, bytes_per_pixel: u32) -> Result<usize, ParseError> {
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(ParseError::InvalidDimensions { width, height });
    }
    let size = (width as u64)
        .checked_mul(height as u64)
        .and_then(|v| v.checked_mul(bytes_per_pixel as u64))
        .ok_or(ParseError::DimensionOverflow { width, height, bytes_per_pixel })?;
    let size = usize::try_from(size).map_err(|_| ParseError::DimensionOverflow { width, height, bytes_per_pixel })?;
    if size > MAX_FRAME_BUFFER_BYTES {
        return Err(ParseError::FrameBufferTooLarge { size, max: MAX_FRAME_BUFFER_BYTES });
    }
    Ok(size)
}
```
- 해상도 필드는 파싱 직후 코덱/프로파일이 정의한 상한과 대조 검증하고, 곱셈은 더 넓은 타입(`u64`)에서 `checked_mul`로 수행한다.
- 계산된 버퍼 크기 자체에도 "이 애플리케이션이 실제로 다룰 실용적 상한"을 정책으로 두어, 스펙상 허용되더라도 비정상적으로 큰 할당 요청은 거부한다.

**탐지 방법**:
- Static: `width * height`, `w * h * bpp` 형태의 곱셈 패턴을 grep해 `checked_mul` 사용 여부 확인, clippy `arithmetic_side_effects` lint 검토.
- Runtime: 해상도 필드를 `0`, `1`, `u32::MAX`, 상한 경계값(`MAX_DIMENSION`, `MAX_DIMENSION + 1`)으로 설정한 fuzz corpus.
- Semantic: 코덱별 스펙에서 정의하는 최대 해상도(프로파일/레벨별로 다름)를 상수 테이블로 정리해 검증 로직과 나란히 문서화.

**예외**:
- 이미 컨테이너 레벨에서 해상도가 검증된 뒤, 같은 함수 내부의 로컬 재계산이라면 중복 검증은 생략 가능(단, 신뢰 경계를 넘나드는 지점 — 컨테이너→코덱 파서 — 에서는 반드시 재검증).

**Bitvue 판정**: N/A (확인, 2026-08-01) — HEVC(`crates/bitvue-hevc/src/sps.rs:335-347`, `MAX_PIC_DIMENSION=16384`), MPEG2(`crates/bitvue-mpeg2-codec/src/sequence.rs:156-168`, `MIN/MAX_MPEG2_DIMENSION` 범위 검증), AVC/VVC/AV1/AV3(각 sps.rs/sequence.rs/sequence_header.rs에 유사한 `MAX_.*DIMENSION`/`MAX_WIDTH` 상수 존재, grep으로 확인)까지 폭넓게 해상도 필드를 파싱 직후 상한 검증. HEVC의 `extract_qp_grid`도 `grid_w.checked_mul(grid_h)`(overlay_extraction.rs:170)로 곱셈 자체를 checked 연산으로 수행. 단, AVS3(`crates/bitvue-avs3/src/sequence_header.rs:135-136`)는 width/height를 14비트 필드로 읽어 상한 검증 코드가 별도로 없으나, 14비트 필드 자체가 최대 16383으로 자연히 bound되어 있어 u32 곱셈 overflow 위험은 없음(0 값에 대한 명시적 거부만 없음, overlay_extraction.rs:72에서 0 체크는 존재).

---

### PARSE-016: tile count 검증 누락
**분류**: PARSE · **심각도**: High · **탐지**: Static/Runtime/Semantic

**나쁜 예**:
```rust
fn parse_tile_info(reader: &mut BitReader, frame_width_sb: u32, frame_height_sb: u32) -> Result<TileInfo, ParseError> {
    let tile_cols_log2 = reader.read_bits(4)?; // 검증 없이 바로 사용
    let tile_rows_log2 = reader.read_bits(4)?;
    let tile_cols = 1u32 << tile_cols_log2; // frame_width_sb와의 관계 검증 없음
    let tile_rows = 1u32 << tile_rows_log2;
    Ok(TileInfo { tile_cols, tile_rows })
}
```

**문제**:
- `tile_cols_log2`/`tile_rows_log2`는 스펙상 `frame_width_sb`/`frame_height_sb`(프레임을 super block 단위로 나눈 크기)에 의해 상한이 결정되는데, 이 관계를 검증하지 않으면 프레임보다 훨씬 많은 타일을 선언하는 malformed 스트림을 그대로 받아들인다.
- 이후 "타일마다 독립적으로 파싱/디코딩"하는 병렬 처리 로직이 `tile_cols * tile_rows`개의 작업 항목을 생성하면, PARSE-014와 동일하게 과도한 리소스 할당(스레드/버퍼)으로 이어진다.
- 타일 경계 정보가 실제 프레임 크기와 불일치하면, 이후 타일별 substream 크기 계산(PARSE-025 참고)도 함께 어긋나 연쇄적으로 잘못된 오프셋에서 파싱을 시도하게 된다.

**발생 조건**:
- AV1 `tile_info()`처럼 "이 값이 유효하려면 프레임 크기에서 유도되는 범위 안에 있어야 한다"는 교차 필드 제약이 있는 신택스에서, 그 제약을 검증하지 않고 값만 읽어 사용할 때.
- HEVC/VVC의 tile column/row 경계 배열도 마찬가지로 "합이 프레임 크기와 일치해야 한다"는 제약이 있는데 이를 생략하면 동일한 문제가 재발한다.

**권장**:
```rust
fn parse_tile_info(reader: &mut BitReader, frame_width_sb: u32, frame_height_sb: u32) -> Result<TileInfo, ParseError> {
    let max_tile_cols_log2 = tile_log2(1, frame_width_sb.min(MAX_TILE_COLS));
    let tile_cols_log2 = reader.read_bits(4)?;
    if tile_cols_log2 > max_tile_cols_log2 {
        return Err(ParseError::TileCountExceedsFrame {
            field: "tile_cols_log2", value: tile_cols_log2, max: max_tile_cols_log2,
        });
    }
    let max_tile_rows_log2 = tile_log2(1, frame_height_sb.min(MAX_TILE_ROWS));
    let tile_rows_log2 = reader.read_bits(4)?;
    if tile_rows_log2 > max_tile_rows_log2 {
        return Err(ParseError::TileCountExceedsFrame {
            field: "tile_rows_log2", value: tile_rows_log2, max: max_tile_rows_log2,
        });
    }
    let tile_cols = 1u32 << tile_cols_log2;
    let tile_rows = 1u32 << tile_rows_log2;
    if tile_cols > frame_width_sb || tile_rows > frame_height_sb {
        return Err(ParseError::TileGridExceedsFrame { tile_cols, tile_rows, frame_width_sb, frame_height_sb });
    }
    Ok(TileInfo { tile_cols, tile_rows })
}
```
- 타일 개수/경계는 항상 "이미 알려진 프레임 크기"라는 문맥 값과 함께 검증한다 — 필드 하나만 독립적으로 유효 범위를 갖는 게 아니라 교차 필드 제약이 있음을 파서가 알고 있어야 한다.
- 스펙의 `tile_log2()` 같은 유도 함수를 그대로 구현해 "허용 가능한 최대 log2 값"을 명시적으로 계산하고, 그 값을 넘는 필드를 거부한다.

**탐지 방법**:
- Semantic: 코덱 스펙 문서에서 "필드 A의 유효 범위가 필드 B에 의존한다"는 교차 필드 제약을 전수 정리하고, 각각에 대응하는 검증 코드가 있는지 대조.
- Runtime: `frame_width_sb`는 작게, `tile_cols_log2`는 최대로 설정하는 조합을 fuzz corpus/unit test로 포함.
- Structural: 타일/서브스트림 관련 파싱 함수가 프레임 크기 컨텍스트를 인자로 받고 있는지(받지 않는다면 애초에 교차 검증이 불가능하므로 설계 결함) 아키텍처 리뷰.

**예외**:
- 없음 — 교차 필드 제약이 있는 필드는 항상 관련 컨텍스트와 함께 검증해야 한다.

**Bitvue 판정**: Suspected (부분 확인, 2026-08-01) — `crates/bitvue-av1-codec/src/tile/tile_group.rs`의 `TileInfo::new()`는 `MAX_TILE_COLS`/`MAX_TILE_ROWS`(64)와 `MAX_TOTAL_TILES`(1024) 절대 상한으로 DoS는 방지하지만, 실제 AV1 스펙의 `tile_info()` 신택스(비트스트림에서 `tile_cols_log2`/`tile_rows_log2`를 읽어 `frame_width_sb`/`frame_height_sb`와 교차검증하는 부분)를 구현한 코드를 찾지 못했고 주석에 "For MVP, we'll use simplified defaults"라고 명시되어 있어(tile_group.rs:21) 이 항목이 지적하는 교차 필드 검증 자체가 아직 구현되지 않은 것으로 보임. HEVC/VVC의 타일 경계 배열 검증은 미확인.

---

### PARSE-017: timestamp arithmetic overflow
**분류**: PARSE · **심각도**: Medium · **탐지**: Static/Runtime

**나쁜 예**:
```rust
fn compute_presentation_time(base_pts: i64, timescale: u32, duration_units: u32) -> i64 {
    base_pts + (duration_units as i64 * 90000 / timescale as i64) // 곱셈이 i64라도 여전히 overflow 가능
}
```

**문제**:
- `duration_units`가 malformed 컨테이너에서 극단값(예: `u32::MAX`)이면 `duration_units as i64 * 90000`이 `i64` 범위조차 넘을 수 있다(약 9.2 × 10^18이 상한이므로 실제로는 `duration_units`가 극단적이어야 하지만, 반복 누적되는 `base_pts`와 결합하면 더 쉽게 넘는다).
- `timescale`이 0이면(malformed 컨테이너 헤더) 나눗셈에서 즉시 panic한다 — 이는 PARSE-006과 겹치는 문제지만 timestamp 계산에서 특히 자주 누락된다.
- 오버플로우된 타임스탬프는 UI에서 "재생 시간이 음수" 또는 "말도 안 되게 먼 미래"로 표시되는 형태로 나타나 사용자를 혼란시키고, 프레임 정렬/탐색(seek) 로직이 이 값을 신뢰하면 잘못된 프레임을 찾아간다.

**발생 조건**:
- 컨테이너의 `timescale`/`duration` 필드가 손상되었거나 0으로 설정된 파일.
- 장시간 스트림에서 PTS/DTS가 누적되어 점진적으로 오버플로우 경계에 접근하는 경우(특히 낮은 timescale에 큰 duration 단위를 곱하는 코덱 조합).

**권장**:
```rust
fn compute_presentation_time(base_pts: i64, timescale: u32, duration_units: u32) -> Result<i64, ParseError> {
    if timescale == 0 {
        return Err(ParseError::InvalidTimescale);
    }
    let delta = (duration_units as i64)
        .checked_mul(90_000)
        .ok_or(ParseError::TimestampOverflow)?
        .checked_div(timescale as i64)
        .ok_or(ParseError::TimestampOverflow)?;
    base_pts.checked_add(delta).ok_or(ParseError::TimestampOverflow)
}
```
- `timescale == 0`을 명시적으로 거부하고, 모든 산술 연산을 `checked_*`로 감싸 오버플로우를 에러로 표면화한다.
- 타임스탬프처럼 "UI에 그대로 노출되는" 값은 특히 신뢰 경계를 넘길 때(파서 → UI 레이어) sanity range(예: 0 ~ 24시간 상당)로 한 번 더 검증하면 사용자에게 이상값이 그대로 노출되는 것을 막을 수 있다.

**탐지 방법**:
- Static: 타임스탬프/duration 계산에 관여하는 산술 연산자를 grep해 `checked_*` 사용 여부 확인.
- Runtime: `timescale = 0`, `duration_units = u32::MAX`, 장시간 스트림 시뮬레이션(반복 누적)을 fuzz/unit 테스트에 포함.
- Semantic: 계산된 타임스탬프가 스트림 전체 길이 대비 합리적 범위인지 검증하는 sanity-check 계층을 UI 경계에 둔다.

**예외**:
- 이미 컨테이너 파싱 단계에서 `timescale > 0`이 스키마 수준으로 보장된 구조(예: 강타입 non-zero 타입을 사용하는 경우)라면 반복 검증은 생략 가능.

**Bitvue 판정**: Confirmed (확인, 2026-08-01) — `crates/bitvue-formats/src/mp4.rs`의 `calculate_timestamps()`(라인 971-978)가 `timestamp += *duration as u64;`로 DTS를 누적할 때 checked/saturating 연산 없이 원시 `+=`를 사용 — 이 문서의 나쁜 예와 동일 패턴. 바로 다음 함수인 `calculate_presentation_timestamps()`(라인 981-999)는 대조적으로 `dts.saturating_add(offset as u64)`/`saturating_sub`를 사용해 동일 파일 내에서도 방어 수준이 일관되지 않음. 다만 `sample_durations`가 `MAX_ENTRY_COUNT`(1000만) 이하로 제한되고 각 duration이 u32이므로 실제 u64 오버플로우 도달은 사실상 불가능(최대 약 4.3×10^16 << u64::MAX)해 실질 익스플로잇 가능성은 낮음. `timescale == 0` 가드는 mp4.rs 내에서 발견하지 못함.

---

### PARSE-018: signed/unsigned 변환 오류
**분류**: PARSE · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
fn read_se(reader: &mut BitReader) -> Result<i32, ParseError> {
    let code_num = reader.read_ue()?;
    // se(v) 변환 공식을 부호 없는 값에 그대로 캐스팅
    let value = (code_num as i32 + 1) / 2;
    Ok(if code_num % 2 == 0 { -value } else { value })
}

fn apply_qp_delta(base_qp: u8, delta: i32) -> u8 {
    (base_qp as i32 + delta) as u8 // 음수가 되면 u8로 wrap되어 거대한 양수가 됨
}
```

**문제**:
- `read_se`의 `code_num as i32` 캐스팅은 `code_num`이 `i32::MAX`를 넘는 큰 `u32`일 때 음수로 뒤집혀버려, 이후 부호 변환 공식 자체가 스펙과 다른 값을 만든다.
- `apply_qp_delta`에서 `base_qp as i32 + delta`가 음수가 되면 `as u8` 캐스팅이 2의 보수 wrap으로 거대한 양수(예: `-1 as u8 == 255`)를 만들어, "QP가 음수라 유효하지 않다"는 논리적 에러가 "QP가 255"라는 완전히 다른(하지만 타입상 유효해 보이는) 값으로 둔갑한다.
- 이런 wrap은 컴파일도, 대부분의 테스트도 조용히 통과시키기 때문에 코드 리뷰에서 캐스팅 방향(부호 있음 → 없음, 넓은 타입 → 좁은 타입)을 명시적으로 짚지 않으면 놓치기 쉽다.

**발생 조건**:
- `se(v)` Exp-Golomb 부호 변환처럼 스펙 자체가 "부호 없는 코드값에서 부호 있는 값을 유도"하는 공식을 가진 필드.
- QP delta, motion vector delta처럼 "기준값 + 오프셋"으로 계산되는 필드에서 오프셋이 음수가 될 수 있는데 결과를 부호 없는 타입에 담을 때.

**권장**:
```rust
fn read_se(reader: &mut BitReader) -> Result<i32, ParseError> {
    let code_num = reader.read_ue()?; // u32
    if code_num > (i32::MAX as u32 - 1) / 2 {
        return Err(ParseError::SignedValueOverflow { code_num });
    }
    let value = (code_num as i32 + 1) / 2;
    Ok(if code_num % 2 == 0 { -value } else { value })
}

fn apply_qp_delta(base_qp: u8, delta: i32) -> Result<u8, ParseError> {
    let result = base_qp as i32 + delta;
    u8::try_from(result).map_err(|_| ParseError::QpOutOfRange { result })
    // 필요하다면 QP 유효 범위(예: 0..=51)로 추가 clamp/검증
}
```
- 부호 있는/없는 변환이 발생하는 모든 지점에서 `as` 캐스팅 대신 `try_from`을 사용해 실패를 명시적으로 처리한다.
- 도메인상 유효 범위(QP는 보통 0~51 또는 코덱별 상수 범위)가 있다면 타입 변환 성공 여부와 별개로 그 범위까지 검증한다.

**탐지 방법**:
- Static: clippy `cast_possible_wrap`, `cast_sign_loss`, `cast_possible_truncation` lint를 파서 크레이트에서 `warn`이 아니라 `deny`로 격상.
- Runtime: 부호 변환이 일어나는 필드에 경계값(`0`, `i32::MAX`, `u32::MAX`)을 넣는 unit/fuzz 테스트.
- Structural: `as i32`, `as u8`, `as u32` 등 캐스팅 지점을 grep해 부호/폭이 바뀌는 캐스팅만 별도 목록화 후 개별 검토.

**예외**:
- 캐스팅 전후 값의 범위가 타입 정의상 이미 안전함이 자명한 경우(예: `u8 as u32`처럼 항상 값 손실이 없는 확장 캐스팅)는 문제 없음.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-core/src/bitreader.rs`의 `ExpGolombReader::read_se()`(라인 609-613)는 `ue.div_ceil(2) as i32` 캐스팅을 사용하지만, 같은 트레이트의 `read_ue()`가 `MAX_EXP_GOLOMB_ZEROS=31` 상한을 강제하므로 `ue`의 최댓값은 2^32-2(4294967294)로 제한되고 `div_ceil(2)`의 최댓값은 정확히 `i32::MAX`(2147483647)와 일치 — 즉 `as i32` 캐스팅이 수학적으로 절대 overflow하지 않는 구조. 모든 `apply_qp_delta`류 다른 캐스팅 지점을 전수 확인하지는 못했으나, 핵심 `se(v)` 구현은 이 항목이 우려하는 버그로부터 안전함이 증명 가능.

---

### PARSE-019: unknown syntax를 즉시 fatal error 처리
**분류**: PARSE · **심각도**: Medium · **탐지**: Structural/Manual

**나쁜 예**:
```rust
fn parse_nal_unit(nal_type: u8, data: &[u8]) -> Result<NalUnit, ParseError> {
    match nal_type {
        NAL_SPS => Ok(NalUnit::Sps(parse_sps(data)?)),
        NAL_PPS => Ok(NalUnit::Pps(parse_pps(data)?)),
        NAL_SLICE => Ok(NalUnit::Slice(parse_slice(data)?)),
        _ => Err(ParseError::UnknownNalType(nal_type)), // 알 수 없으면 전체 파싱 중단
    }
}
```

**문제**:
- 코덱 스펙은 계속 확장되며(새 프로파일, 새 SEI 타입, 벤더 확장 NAL 등), 파서가 "이해하지 못하는 신택스 = 즉시 fatal"로 처리하면 최신 인코더가 만든 정상 파일조차 "미지원 NAL 하나" 때문에 전체 분석이 중단된다.
- 분석 도구의 목적상, SEI 같은 부가 정보나 미래 확장 NAL은 "건너뛰고 나머지는 계속 분석"하는 것이 사용자에게 훨씬 유용하다 — 특히 비트스트림 분석기는 "이해 못하는 것도 최대한 위치와 크기만이라도 보여주는" 관대함이 요구사항에 가깝다.
- 반대로 모든 unknown syntax를 무조건 무시하는 것도 위험하다 — 신택스 요소가 실제로 이후 비트 위치에 영향을 주는데(가변 길이 필드) 이를 그냥 건너뛰면 이후 파싱이 어긋난다. "안전하게 건너뛸 수 있는 경우"와 "건너뛸 수 없는 경우"를 구분해야 한다.

**발생 조건**:
- 최신 코덱 확장(예: HEVC의 새로운 SEI payload type, AV1의 새로운 OBU 타입, VVC의 벤더별 확장)을 아직 구현하지 않은 파서 버전으로 최신 인코더 출력을 분석할 때.
- 스펙에 정의되지 않은 벤더 확장이나 실험적 신택스가 섞인 파일.

**권장**:
```rust
fn parse_nal_unit(nal_type: u8, data: &[u8]) -> Result<NalUnit, ParseError> {
    match nal_type {
        NAL_SPS => Ok(NalUnit::Sps(parse_sps(data)?)),
        NAL_PPS => Ok(NalUnit::Pps(parse_pps(data)?)),
        NAL_SLICE => Ok(NalUnit::Slice(parse_slice(data)?)),
        // NAL 단위처럼 자기 자신의 길이가 컨테이너/스타트코드로 이미 구획된 구조는
        // 내용을 이해 못해도 "위치+크기+raw bytes"만 보존하고 건너뛸 수 있다
        other if is_reserved_or_unknown(other) => Ok(NalUnit::Unrecognized {
            nal_type: other,
            raw: data.to_vec(),
        }),
        _ => Err(ParseError::UnknownNalType(nal_type)),
    }
}
```
- 신택스 요소가 "자체 길이 정보를 갖고 있어 내용을 몰라도 안전하게 스킵 가능한 구조"(대부분의 NAL/OBU는 컨테이너 또는 start code로 경계가 정해짐)인지 먼저 판별한다.
- 안전하게 스킵 가능하면 `Unrecognized { type, raw_bytes }` 같은 변형으로 보존해, 사용자가 "이건 우리가 모르는 신택스"라는 사실 자체를 확인할 수 있게 한다(완전 무시하지 않음).
- 반대로 길이 경계가 없는(비트 단위로 계속 이어지는) 신택스에서 unknown 필드를 만나면, 그 지점 이후는 정말로 신뢰할 수 없으므로 fatal 처리하거나 "이 지점부터 확신도 낮음" 마킹을 남긴다.

**탐지 방법**:
- Structural: `match nal_type { ... _ => Err(...) }` 형태의 catch-all 에러 패턴을 grep해, 스킵 가능한 구조인지 개별 검토.
- Manual: 최신 스펙 개정판/최신 인코더(x265, SVT-AV1, VVenC 등) 출력물을 정기적으로 회귀 corpus에 추가해 "새로 추가된 신택스 때문에 전체 파싱이 깨지는지" 확인.

**예외**:
- 보안이 최우선인 컨텍스트(예: 디코딩 파이프라인 자체)에서는 unknown syntax를 관대하게 처리하는 것이 오히려 위험할 수 있다 — 다만 이 카탈로그의 대상은 "분석 도구"이므로 관대한 처리가 기본 원칙이다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-hevc/src/nal.rs`의 `NalUnitType`은 알 수 없는 값을 `Unspec63`(catch-all "RESERVED/UNSPEC") variant로 매핑(라인 144, 244)해 fatal error 없이 보존; `crates/bitvue-formats/src/mp4.rs`의 최상위 box 순회 루프도 알 수 없는 box type을 `_ => { /* Skip unknown box */ }`(라인 407-410)로 건너뛰고 전체 파싱을 계속 진행. 이 항목이 우려하는 "즉시 fatal" 패턴은 확인한 두 경로 모두에서 발견되지 않음.

---

### PARSE-020: reserved bit 처리 정책 불명확
**분류**: PARSE · **심각도**: Low · **탐지**: Structural/Manual

**나쁜 예**:
```rust
fn parse_nal_header(reader: &mut BitReader) -> Result<NalHeader, ParseError> {
    let forbidden_zero_bit = reader.read_bit()?;
    if forbidden_zero_bit != 0 {
        return Err(ParseError::ForbiddenBitSet); // 항상 0이어야 하는데 아니면 fatal
    }
    let nal_type = reader.read_bits(6)?;
    let reserved = reader.read_bits(9)?; // 읽기만 하고 검증 정책이 코드 어디에도 없음
    Ok(NalHeader { nal_type })
}
```

**문제**:
- `reserved` 필드를 읽어놓고도 그 값에 대해 아무 정책이 없다 — 나중에 코드를 읽는 사람은 "이 필드를 검증해야 하는지, 무시해도 되는지" 알 수 없다.
- 스펙마다 reserved 필드의 정책이 다르다: 어떤 필드는 "미래 확장을 위해 반드시 0이어야 함"(위반 시 conformance 위반으로 취급), 어떤 필드는 "디코더가 무시해야 함(MUST be ignored)"으로 명시된다. 이 둘을 코드에서 구분하지 않으면, 미래 스펙 확장이 이 필드를 실제로 사용하기 시작했을 때(정상적으로 0이 아닌 값이 들어옴) 앞의 `forbidden_zero_bit`처럼 잘못 fatal 처리할 위험이 있다.
- 반대로 실제로는 "반드시 0이어야 하는" 필드인데 검증을 생략하면, 향후 이 필드가 정말 0이 아닐 때(스트림 손상 신호일 수 있음) 놓치게 된다.

**발생 조건**:
- 코덱 표준 신택스 테이블을 그대로 옮기면서 `reserved_zero_Nbits` 같은 필드를 "일단 읽어서 버리는" 식으로 구현할 때 정책을 문서화하지 않고 넘어가는 경우.
- 스펙 개정판이 이전에는 reserved였던 필드를 실제 신택스로 재정의했을 때(하위 호환성 문제).

**권장**:
```rust
/// reserved_zero_9bits: 스펙 X.Y.Z에 따르면 미래 확장을 위해 예약되었으며,
/// 디코더는 이 필드의 값과 무관하게 무시해야 한다(MUST be ignored by decoders).
/// 다만 분석기는 값이 0이 아닌 경우 "미래 확장 신택스일 가능성"으로 경고만 남긴다.
fn parse_nal_header(reader: &mut BitReader) -> Result<NalHeader, ParseError> {
    let forbidden_zero_bit = reader.read_bit()?;
    if forbidden_zero_bit != 0 {
        return Err(ParseError::ForbiddenBitSet);
    }
    let nal_type = reader.read_bits(6)?;
    let reserved = reader.read_bits(9)?;
    let mut warnings = Vec::new();
    if reserved != 0 {
        warnings.push(ParseWarning::NonZeroReserved { field: "reserved_zero_9bits", value: reserved });
    }
    Ok(NalHeader { nal_type, warnings })
}
```
- 모든 reserved/forbidden 필드에 대해 "스펙이 이 필드를 어떻게 규정하는지"(반드시 0 / MUST ignore / 향후 확장용)를 주석으로 명시하고, 코드의 처리 방식이 그 규정과 일치하게 한다.
- "MUST be 0이 아니면 conformance 위반"인 필드와 "MUST ignore(디코더가 무시해야 함)"인 필드를 별도 헬퍼(`read_forbidden_zero_bit` vs `read_reserved_bits_ignored`)로 구분해, 호출부만 봐도 정책이 드러나게 한다.
- 값이 0이 아닌 reserved 필드를 만나면 fatal이 아니라 warning으로 기록해, 사용자가 "이 스트림이 최신 확장을 쓰는 것 같다"는 신호를 받을 수 있게 한다(PARSE-019와 연계).

**탐지 방법**:
- Manual: 코덱 스펙 신택스 테이블에서 `reserved`/`forbidden`으로 명명된 모든 필드를 목록화하고, 각각의 처리 정책(0 강제 검증 여부, 무시 여부)이 코드 주석에 명시되어 있는지 대조.
- Structural: `read_bits(n)` 호출 결과를 바로 버리는(`let _ = ...` 또는 결과 미사용) 지점을 grep해 정책 문서화 여부 확인.

**예외**:
- 정말로 스펙이 "값과 무관하게 완전히 무시"라고 명시한 필드는 경고조차 남기지 않고 버려도 무방하다 — 다만 그 사실을 주석으로 남기는 것은 여전히 권장된다.

**Bitvue 판정**: Suspected (미확인) — reserved bit 필드가 "MUST be 0"과 "MUST ignore"로 구분되어 처리되는지, 혹은 처리 정책이 주석으로 문서화되어 있는지를 직접 확인하지 못함. `read_forbidden_zero_bit` vs `read_reserved_bits_ignored` 같은 이름이 구분된 헬퍼 함수는 grep으로 찾지 못했으나, 이는 이 항목의 심각도(Low)에 비해 조사 우선순위를 낮춘 결과이지 부재를 확정한 것은 아님.

---

### PARSE-021: trailing bits 검증 누락
**분류**: PARSE · **심각도**: Medium · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
fn parse_pps(reader: &mut BitReader) -> Result<Pps, ParseError> {
    let pps_id = reader.read_ue()?;
    let sps_id = reader.read_ue()?;
    // ... 나머지 필드들 파싱 ...
    // rbsp_trailing_bits()를 확인하지 않고 그냥 반환
    Ok(Pps { pps_id, sps_id /* ... */ })
}
```

**문제**:
- H.264/HEVC 계열 RBSP는 `rbsp_trailing_bits()`(stop bit `1` + 정렬용 `0` 패딩)로 끝나야 하는데, 이를 검증하지 않으면 "실제로는 신택스 요소가 더 있었는데 파서가 일찍 멈춘" 경우와 "정상적으로 끝난" 경우를 구분하지 못한다.
- 파서가 실제 필드 파싱 로직에 버그가 있어 필드를 하나 빠뜨렸을 때, trailing bits 검증이 없으면 "정상 종료"로 착각하고 조용히 잘못된 결과를 반환한다 — trailing bits 검증은 사실상 "내가 이 신택스를 스펙대로 정확히 다 읽었다"는 자체 검증(self-check) 역할을 한다.
- 이 검증이 없으면 SPS/PPS 파싱 로직의 미묘한 오프바이원 버그가 conformance 테스트 없이는 발견되지 않고 프로덕션까지 흘러간다.

**발생 조건**:
- 코덱 신택스 확장(새 프로파일 플래그 등)이 파서에 아직 반영되지 않아 실제 스트림에 남은 비트가 있는데 이를 무시하고 넘어가는 상황.
- 파서 개발 중 필드 하나를 빠뜨리는 흔한 실수(신규 신택스 요소 추가 시 특히 발생하기 쉬움).

**권장**:
```rust
fn parse_pps(reader: &mut BitReader) -> Result<Pps, ParseError> {
    let pps_id = reader.read_ue()?;
    let sps_id = reader.read_ue()?;
    // ... 나머지 필드들 파싱 ...
    reader.verify_rbsp_trailing_bits()?; // stop bit(1) + 0-padding까지 정확히 소비했는지 검증
    Ok(Pps { pps_id, sps_id /* ... */ })
}

impl<'a> BitReader<'a> {
    fn verify_rbsp_trailing_bits(&mut self) -> Result<(), ParseError> {
        let stop_bit = self.read_bit()?;
        if stop_bit != 1 {
            return Err(ParseError::MissingRbspStopBit);
        }
        while !self.is_byte_aligned() {
            if self.read_bit()? != 0 {
                return Err(ParseError::NonZeroAlignmentPadding);
            }
        }
        Ok(())
    }
}
```
- 모든 RBSP 파서(SPS/PPS/VPS/slice header)의 마지막 단계에서 `rbsp_trailing_bits()` 검증을 공통 헬퍼로 강제한다.
- 검증 실패를 fatal로 취급할지 warning으로 남길지는 정책에 따라 다르지만(관대한 파싱 원칙 — PARSE-019), 최소한 "trailing bits가 예상과 다르다"는 사실 자체는 반드시 어딘가에 기록되어야 한다.

**탐지 방법**:
- Structural: RBSP 계열 파서 함수가 모두 공통 `verify_rbsp_trailing_bits` 호출로 끝나는지 코드 리뷰 체크리스트화.
- Runtime: 각 신택스 파서에 대해 "정확히 정답 바이트 수만큼만 소비했는가"를 확인하는 conformance 테스트(공식 conformance 비트스트림 활용).

**예외**:
- Trailing bits 개념이 없는 신택스 구조(길이 접두사가 명시적으로 있는 OBU/박스 등)에는 해당하지 않는다 — 대신 PARSE-025(substream 경계) 원칙이 적용된다.

**Bitvue 판정**: Confirmed (확인, 2026-08-01) — `crates/bitvue-hevc/src/bitreader.rs`와 `crates/bitvue-vvc/src/bitreader.rs`에 `read_rbsp_trailing_bits()` 함수가 각각 정의되어 있으나(hevc 라인 145, vvc 라인 142), `grep -rn "read_rbsp_trailing_bits(" crates/bitvue-hevc/src crates/bitvue-vvc/src`로 실제 호출부를 찾은 결과 자기 자신의 정의 외에는 호출하는 곳이 전혀 없음 — `crates/bitvue-hevc/src/sps.rs`의 `parse_sps`도 마지막 필드 파싱 후 바로 `Ok(Sps{...})`로 반환하며 trailing bits 검증을 거치지 않음. 안전 장치가 구현되어 있음에도 배선(wiring)되지 않아 자체 검증(self-check) 효과가 실질적으로 작동하지 않는 죽은 코드(dead code) 상태.

---

### PARSE-022: alignment bit 처리 오류
**분류**: PARSE · **심각도**: Medium · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
fn parse_slice_data(reader: &mut BitReader) -> Result<SliceData, ParseError> {
    let header = parse_slice_header(reader)?;
    // byte_alignment()를 건너뛰고 바로 다음 바이트 경계에서 읽는다고 가정
    reader.align_to_byte(); // 실제로 몇 비트를 건너뛰는지, 그 비트가 검증되는지 불명확
    let entropy_data = reader.remaining_bytes();
    Ok(SliceData { header, entropy_data })
}
```

**문제**:
- `align_to_byte()`가 "다음 바이트 경계까지 그냥 건너뛰는" 구현이면, 스펙이 요구하는 `alignment_bit_equal_to_one` + `alignment_bit_equal_to_zero*` 패턴(첫 비트는 1, 나머지는 0)을 검증하지 않고 넘어간다 — PARSE-021과 유사하지만 "슬라이스 헤더 → 엔트로피 데이터 경계"처럼 스트림 중간의 정렬에서 특히 중요하다.
- 정렬 비트를 검증 없이 건너뛰면, 슬라이스 헤더 파싱에 오프바이원 버그가 있어도 "우연히 바이트 경계에 도달"하는 바람에 겉보기엔 정상 동작하는 것처럼 보여 버그가 은폐된다.
- CABAC/엔트로피 코딩 데이터의 시작 위치가 1비트라도 어긋나면 이후 모든 계수/모드 디코딩이 완전히 틀어지는데, 이 항목의 검증 누락이 바로 그 어긋남의 흔한 원인이다.

**발생 조건**:
- 슬라이스 헤더에서 엔트로피 코딩된 슬라이스 데이터로 넘어가는 경계(`byte_alignment()` 호출 지점).
- 필터/타일 그룹 헤더처럼 스펙상 중간에 바이트 정렬을 강제하는 여러 지점.

**권장**:
```rust
impl<'a> BitReader<'a> {
    fn byte_alignment(&mut self) -> Result<(), ParseError> {
        let alignment_bit = self.read_bit()?;
        if alignment_bit != 1 {
            return Err(ParseError::MissingAlignmentBit);
        }
        while !self.is_byte_aligned() {
            if self.read_bit()? != 0 {
                return Err(ParseError::NonZeroAlignmentPadding);
            }
        }
        Ok(())
    }
}

fn parse_slice_data(reader: &mut BitReader) -> Result<SliceData, ParseError> {
    let header = parse_slice_header(reader)?;
    reader.byte_alignment()?; // 스펙이 정의한 패턴을 그대로 검증
    let entropy_data = reader.remaining_bytes()?;
    Ok(SliceData { header, entropy_data })
}
```
- "그냥 다음 바이트로 건너뛰기"와 "스펙이 정의한 정렬 비트 패턴을 검증하며 건너뛰기"를 서로 다른 함수로 명확히 구분하고, 스펙이 특정 패턴을 요구하는 지점에서는 반드시 후자를 쓴다.
- 정렬 실패는 그 자체로 "여기까지의 파싱이 잘못되었다"는 강한 신호이므로, 조용히 넘어가지 않고 에러로 표면화한다.

**탐지 방법**:
- Structural: `align_to_byte`/`skip_to_byte_boundary`류 함수가 검증 없는 버전과 검증하는 버전으로 구분되어 있는지, 호출부가 올바른 버전을 쓰는지 리뷰.
- Runtime: 슬라이스 헤더 필드를 하나씩 의도적으로 잘못 파싱하도록 만든 뒤(테스트 전용 mutation), 정렬 검증이 그 오류를 잡아내는지 확인하는 mutation testing.

**예외**:
- 스펙이 "단순 스킵"만 요구하는 정렬 지점(값 검증 없이 그냥 바이트 경계 이동)이라면 검증 없는 스킵도 정당하다 — 각 지점마다 스펙 문구를 확인해야 한다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-core/src/bitreader.rs`의 `byte_align()`(라인 336-341)은 이 항목의 나쁜 예와 동일하게 정렬 비트 패턴을 검증하지 않는 단순 스킵 구현이지만, `grep -rn "\.byte_align()"`으로 확인한 결과 HEVC/AVC/VVC 어느 코덱 크레이트의 실제 파싱 로직에서도 이 메서드가 호출되지 않음(wrapper 정의 내부 위임 호출만 존재) — 즉 슬라이스 헤더→슬라이스 데이터 경계처럼 이 항목이 우려하는 지점 자체가 현재 구현에서 실행되지 않아(관련 CABAC 슬라이스 데이터 디코딩이 헤더 레벨까지만 구현됨) 위험이 발동하지 않음.

---

### PARSE-023: EBSP→RBSP 변환마다 allocation
**분류**: PARSE · **심각도**: Medium · **탐지**: Static/Runtime

**나쁜 예**:
```rust
fn nal_to_rbsp(ebsp: &[u8]) -> Vec<u8> {
    let mut rbsp = Vec::new(); // 용량 힌트 없음
    let mut zero_count = 0;
    for &byte in ebsp {
        if zero_count >= 2 && byte == 0x03 {
            zero_count = 0;
            continue; // emulation prevention byte 제거
        }
        rbsp.push(byte);
        zero_count = if byte == 0 { zero_count + 1 } else { 0 };
    }
    rbsp
}

fn parse_all_nals(nals: &[&[u8]]) -> Vec<Sps> {
    nals.iter().map(|n| {
        let rbsp = nal_to_rbsp(n); // NAL마다 새 Vec 할당 — 수천 개 NAL이면 수천 번 할당
        parse_sps(&rbsp).unwrap()
    }).collect()
}
```

**문제**:
- `Vec::new()`로 시작하면 `push`할 때마다 재할당/복사가 여러 번 일어날 수 있고(용량이 부족할 때마다 2배 growth), emulation prevention byte 제거처럼 "원본보다 살짝 작아지는" 변환에 이 비효율이 누적된다.
- 대용량 스트림(수만~수십만 NAL)을 프레임 단위로 순회하며 매번 EBSP→RBSP 변환을 새로 할당하면, 분석 도구의 "파일을 열고 스캔하는" 첫 단계 자체가 체감될 정도로 느려진다 — 특히 UI 스레드에서 동기적으로 수행되면 응답성 문제로 직결된다.
- 이 항목은 안전성보다는 성능/확장성 문제지만, 대용량 파일을 다루는 분석 도구의 핵심 경로(hot path)이므로 카탈로그에 포함할 가치가 있다.

**발생 조건**:
- 전체 파일을 열자마자 모든 NAL/OBU를 미리 스캔하는 "eager materialization"(PARSE-030과 연관) 초기 로딩 단계.
- 슬라이더로 프레임을 빠르게 탐색(scrub)할 때마다 근처 NAL들을 재변환하는 경우.

**권장**:
```rust
fn nal_to_rbsp_into(ebsp: &[u8], rbsp: &mut Vec<u8>) {
    rbsp.clear();
    rbsp.reserve(ebsp.len()); // 최악의 경우(제거되는 바이트가 0개)를 상한으로 미리 예약
    let mut zero_count = 0;
    for &byte in ebsp {
        if zero_count >= 2 && byte == 0x03 {
            zero_count = 0;
            continue;
        }
        rbsp.push(byte);
        zero_count = if byte == 0 { zero_count + 1 } else { 0 };
    }
}

fn parse_all_nals(nals: &[&[u8]]) -> Result<Vec<Sps>, ParseError> {
    let mut scratch = Vec::new(); // 버퍼를 재사용
    nals.iter().map(|n| {
        nal_to_rbsp_into(n, &mut scratch);
        parse_sps(&scratch)
    }).collect()
}
```
- 변환 결과 크기의 상한(원본 크기, emulation prevention byte 제거는 크기를 줄이기만 하므로 원본 길이가 항상 안전한 상한)으로 `reserve`해 재할당 횟수를 최소화한다.
- 반복 호출되는 경로에서는 출력 버퍼를 호출자가 재사용할 수 있는 `_into` 스타일 API를 제공해, 매 호출마다 새 `Vec`을 만들지 않게 한다.
- 정말 성능이 중요한 경로에서는 EBSP를 아예 복사하지 않고 emulation prevention byte 위치만 기록해 "가상 RBSP 뷰"를 제공하는 방식(zero-copy)도 고려할 수 있다(PARSE-024와 함께 검토).

**탐지 방법**:
- Runtime: flamegraph/perf로 파일 열기 단계의 할당 hotspot을 프로파일링(`cargo flamegraph`, `dhat` 등으로 allocation count 측정).
- Static: `Vec::new()` 뒤에 루프에서 `push`하는 패턴을 grep해 `with_capacity`/`reserve` 사용 여부 점검.

**예외**:
- 파일 전체가 아니라 사용자가 명시적으로 선택한 소수의 NAL만 온디맨드로 변환하는 경로라면, 이 최적화의 이득이 작아 우선순위가 낮다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-core/src/bitreader.rs`의 `remove_emulation_prevention_bytes()`(라인 790-818)가 `Vec::with_capacity(data.len())`(라인 802)로 원본 크기를 상한으로 사전 예약해 이 문서가 지적하는 "Vec::new() 후 반복 push로 인한 재할당 누적" 문제를 이미 회피. 다만 버퍼 재사용(`_into` 스타일 API)까지는 구현되어 있지 않아 NAL마다 새 `Vec`을 할당하긴 함(PARSE-024/030과 연관).

---

### PARSE-024: OBU payload 매번 복사
**분류**: PARSE · **심각도**: Medium · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
struct Obu {
    obu_type: u8,
    payload: Vec<u8>, // 항상 원본 버퍼에서 복사해서 소유
}

fn parse_obu(data: &[u8]) -> Result<Obu, ParseError> {
    let obu_type = data[0] >> 3;
    let payload = data[1..].to_vec(); // 조회/표시만 할 뿐인데 매번 복사
    Ok(Obu { obu_type, payload })
}
```

**문제**:
- 분석 도구의 전형적인 사용 패턴은 "OBU를 순회하며 타입/크기만 보여주고, 사용자가 클릭한 것만 상세 파싱"인데, 모든 OBU의 payload를 미리 `to_vec()`으로 복사하면 파일 전체 크기만큼의 메모리를 즉시 이중으로(원본 파일 버퍼 + 복사본) 점유한다.
- 대용량 파일(수 GB)을 열 때 이 복사 비용이 열기 시간과 메모리 사용량을 모두 크게 늘려, PARSE-030(eager materialization)과 함께 대용량 파일 지원의 실질적 병목이 된다.
- 원본 파일 버퍼가 이미 메모리에 매핑되어 있거나(mmap) `Vec<u8>`로 로드되어 있다면, payload는 그 버퍼의 슬라이스로 표현 가능한데도 굳이 소유권을 복제하는 것은 불필요한 설계다.

**발생 조건**:
- 파일을 열자마자 모든 OBU/NAL 목록을 UI 트리에 표시하기 위해 미리 전부 파싱하는 초기 로딩 단계.
- 여러 OBU를 비교하거나 hex view에서 원본 바이트를 보여줄 때, 이미 존재하는 원본 버퍼 대신 복사본을 다시 참조하는 경우.

**권장**:
```rust
struct Obu<'a> {
    obu_type: u8,
    payload: &'a [u8], // 원본 버퍼를 빌려서 참조 — 복사 없음
}

fn parse_obu(data: &[u8]) -> Result<Obu<'_>, ParseError> {
    let obu_type = data.first().ok_or(ParseError::UnexpectedEof { needed: 1, remaining: 0 })? >> 3;
    let payload = data.get(1..).ok_or(ParseError::OutOfBounds { offset: 1, end: 1, data_len: data.len() })?;
    Ok(Obu { obu_type, payload })
}
```
- OBU/NAL 목록을 만드는 1차 스캔 단계에서는 `&'a [u8]` 슬라이스(빌림)로 구조체를 정의해, 원본 파일 버퍼의 라이프타임에 묶인 zero-copy 뷰만 유지한다.
- 사용자가 실제로 상세 파싱(신택스 트리 전개)을 요청한 OBU에 한해서만, 필요하다면 그 시점에 복사하거나 그대로 참조를 파싱에 사용한다.
- 파일 전체가 메모리 매핑(mmap) 가능한 크기라면, 애초에 원본을 `Vec<u8>`로 읽어들이지 않고 `memmap2` 같은 크레이트로 매핑해 OS가 페이지 단위로 지연 로딩하게 하는 것도 대용량 파일 대응에 유효하다.

**탐지 방법**:
- Static: 파서 구조체 정의에서 `payload: Vec<u8>` 대신 `payload: &'a [u8]`를 쓸 수 있는지 리뷰(라이프타임 도입이 API를 복잡하게 만드는 트레이드오프도 함께 고려).
- Runtime: 대용량 파일(수 GB) 로딩 시 피크 메모리 사용량을 측정해 "파일 크기의 몇 배"가 소모되는지 벤치마크로 추적.

**예외**:
- 원본 버퍼가 파싱 도중 해제되거나 재사용될 수 있는 스트리밍 파이프라인(예: 네트워크 청크 단위 파싱)에서는 참조 대신 소유권 있는 복사가 오히려 올바른 선택일 수 있다 — zero-copy는 "원본이 파싱 결과의 라이프타임보다 오래 살아있음을 보장할 수 있을 때"만 유효하다.

**Bitvue 판정**: Confirmed (확인, 2026-08-01) — `crates/bitvue-hevc/src/nal.rs`, `crates/bitvue-avc/src/nal.rs`, `crates/bitvue-vvc/src/nal.rs`의 `NalUnit` 구조체가 `payload: Vec<u8>`와 `raw_payload: Vec<u8>` 둘 다 소유(hevc 라인 275-278 등) — 즉 NAL 하나당 원본 버퍼로부터 두 벌의 owned 복사본을 만듦. `parse_nal_units()`(hevc 라인 425-449, avc/vvc도 동일 구조)가 파일 전체의 모든 NAL에 대해 `nal_data[2..].to_vec()` + `remove_emulation_prevention_bytes()`를 즉시 수행해 `&'a [u8]` 기반 zero-copy 구조체는 채택되지 않음. AV1(`crates/bitvue-av1-codec/src/obu.rs:146,316`)은 `payload: Arc<[u8]>`를 사용해 최초 1회 복사 이후 clone 비용은 낮지만, 원본 파일 버퍼로부터의 최초 복사 자체는 여전히 발생해 완전한 zero-copy는 아님.

---

### PARSE-025: substream boundary를 부모 reader와 공유
**분류**: PARSE · **심각도**: High · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
fn parse_tile_group(reader: &mut BitReader, tile_size: u32) -> Result<TileData, ParseError> {
    // tile_size만큼만 읽어야 하는데, 부모 reader를 그대로 넘겨 계속 사용
    let tile_data = parse_tile(reader)?; // parse_tile이 tile_size를 넘어서 읽어도 감지 불가
    Ok(tile_data)
}
```

**문제**:
- 타일/서브스트림처럼 "이 구간은 정확히 N바이트여야 한다"는 경계가 스펙상 명시된 구조를, 부모 reader를 그대로 공유해서 파싱하면 자식 파서(`parse_tile`)의 버그(오프바이원, 필드 누락)로 인해 경계를 넘어 다음 타일의 데이터까지 침범해 읽어도 아무도 감지하지 못한다.
- 이런 경계 침범은 "다음 타일이 이상한 값으로 시작하는" 형태로 나타나는데, 원인은 실제로는 이전 타일 파서에 있어 디버깅이 매우 어렵다 — 증상과 원인의 위치가 멀리 떨어져 있다.
- 여러 타일을 병렬로 파싱하려는 시도(성능 최적화) 자체가, 애초에 각 타일이 독립된 경계를 가진 서브 reader로 분리되어 있지 않으면 불가능하다 — 이 항목은 안전성과 병렬화 가능성을 동시에 막는 설계 결함이다.

**발생 조건**:
- AV1의 타일 그룹, HEVC/VVC의 타일/서브픽처, 여러 서브스트림으로 나뉘는 컨테이너 구조(예: MKV의 lacing된 블록)를 파싱할 때.
- "일단 빨리 동작하게 하자"는 압박 속에서 서브 reader를 새로 만드는 대신 부모 reader를 그대로 넘기는 지름길을 택했을 때.

**권장**:
```rust
fn parse_tile_group(reader: &mut BitReader, tile_size_bytes: u32) -> Result<TileData, ParseError> {
    let (byte_offset, _) = reader.byte_position()?; // 현재 바이트 정렬 위치
    let end = byte_offset
        .checked_add(tile_size_bytes as usize)
        .ok_or(ParseError::OffsetOverflow { offset: byte_offset, length: tile_size_bytes as usize })?;
    let tile_bytes = reader
        .underlying_bytes()
        .get(byte_offset..end)
        .ok_or(ParseError::OutOfBounds { offset: byte_offset, end, data_len: reader.underlying_bytes().len() })?;

    let mut sub_reader = BitReader::new(tile_bytes); // 독립된 경계를 가진 서브 reader
    let tile_data = parse_tile(&mut sub_reader)?;
    // 자식이 경계를 다 쓰지 않았어도, 부모는 항상 정확히 tile_size_bytes만큼 전진
    reader.seek_to_byte(end)?;
    Ok(tile_data)
}
```
- 스펙이 명시적 길이를 부여한 서브스트림은 항상 그 길이만큼만 볼 수 있는 독립된 서브 reader(부모 버퍼의 슬라이스)를 만들어 자식 파서에 넘긴다 — 자식이 아무리 버그가 있어도 그 경계를 물리적으로 넘을 수 없게 만든다.
- 자식 파싱이 끝난 뒤 부모 reader는 자식이 실제로 얼마나 읽었는지와 무관하게 항상 선언된 길이만큼 전진시켜, "자식이 덜 읽었다"/"더 읽으려 했다"는 사실이 오류로 이어지지 않고 다음 서브스트림이 항상 올바른 위치에서 시작하게 한다.
- 이 구조는 부수적으로 타일 병렬 파싱(서로 다른 서브 reader를 독립 스레드에서 처리)도 자연스럽게 가능하게 한다.

**탐지 방법**:
- Structural: 스펙상 명시적 길이가 있는 서브스트림을 파싱하는 함수가 독립된 `BitReader`/서브슬라이스를 생성하는지, 아니면 부모 reader를 그대로 넘기는지 아키텍처 리뷰로 전수 점검.
- Runtime: 특정 타일 파서가 의도적으로 선언된 크기보다 더 많이/적게 읽도록 만든 fuzz/unit 테스트로, 그 오류가 다른 타일에 전파되지 않는지 검증.

**예외**:
- 서브스트림 경계가 스펙상 명시되지 않고 "다음 구조가 시작되는 지점까지"로만 암묵적으로 정의되는 경우(경계 자체를 파싱으로 알아내야 하는 구조)는 이 패턴을 그대로 적용하기 어렵다 — 이런 구조는 애초에 파서 설계 단계에서 별도로 신중히 다뤄야 한다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-av1-codec/src/tile/tile_group.rs`의 `parse_tile_group_full()`(라인 194-249)이 이 항목의 "권장" 코드와 거의 동일한 패턴: 각 타일마다 `tile_size`를 LEB128로 읽어 `if offset + tile_size > data.len() { return Err(...) }`로 경계를 검증한 뒤 `data[offset..offset+tile_size].to_vec()`로 완전히 독립된 슬라이스를 만들고, `offset += tile_size`로 자식 파서의 실제 소비량과 무관하게 항상 선언된 크기만큼 전진(라인 244-248). 컨테이너 레벨에서도 `crates/bitvue-formats/src/mp4.rs`의 각 box 순회 루프가 자식 박스 파싱 결과와 무관하게 항상 `cursor.seek(SeekFrom::Start(child_end))`로 선언된 경계까지 전진(라인 487 등)해 동일 원칙을 적용.

---

### PARSE-026: EOF를 0비트처럼 처리
**분류**: PARSE · **심각도**: Critical · **탐지**: Static/Runtime

**나쁜 예**:
```rust
impl<'a> BitReader<'a> {
    fn read_bit(&mut self) -> u8 {
        if self.bit_pos / 8 >= self.data.len() {
            return 0; // EOF에서 그냥 0을 반환 — 정상 데이터와 구분 불가
        }
        let byte = self.data[self.bit_pos / 8];
        let bit = (byte >> (7 - self.bit_pos % 8)) & 1;
        self.bit_pos += 1;
        bit
    }
}
```

**문제**:
- EOF 상태와 "실제로 0비트를 읽은 것"이 똑같은 반환값(`0`)으로 표현되면, 호출자는 파일이 잘렸는지 아니면 스트림이 정상적으로 0을 인코딩했는지 전혀 구분할 수 없다.
- PARSE-005에서 본 것처럼, Exp-Golomb의 `while read_bit() == 0` 같은 루프가 EOF에서도 계속 0을 받으면 무한 루프(혹은 `bit_pos`가 끝없이 증가하는 준-무한 루프)에 빠진다 — 이 항목은 그 근본 원인이다.
- 파싱 결과가 "그럴듯하지만 틀린" 값(0으로 채워진 필드들)으로 나오면, 사용자는 파일이 잘렸다는 사실을 인지하지 못한 채 잘못된 분석 결과를 신뢰하게 된다 — 조용한 데이터 손상 중 가장 나쁜 형태다.

**발생 조건**:
- 네트워크 스트리밍 도중 잘린 파일, 다운로드가 중단된 파일, 의도적으로 truncate된 fuzz 입력.
- 컨테이너가 선언한 sample size가 실제 파일에 남은 바이트 수보다 큰 손상 파일(PARSE-036과 연관).

**권장**:
```rust
impl<'a> BitReader<'a> {
    fn read_bit(&mut self) -> Result<u8, ParseError> {
        if self.bit_pos / 8 >= self.data.len() {
            return Err(ParseError::UnexpectedEof { needed: 1, remaining: 0 });
        }
        let byte = self.data[self.bit_pos / 8];
        let bit = (byte >> (7 - self.bit_pos % 8)) & 1;
        self.bit_pos += 1;
        Ok(bit)
    }
}
```
- EOF는 항상 명시적인 `Err`로 반환하고, 이를 정상 값(`0`)과 절대 혼동하지 않는다 — `Option<u8>`이나 전용 `ParseError::UnexpectedEof` variant를 사용한다.
- 이 원칙 하나만 지켜도 PARSE-002, PARSE-005 같은 여러 무한 루프/오탐 문제가 근본적으로 예방된다 — bit-reader의 EOF 처리는 파서 안전성의 기초 중의 기초다.

**탐지 방법**:
- Static: bit-reader 구현에서 EOF 검사 후 반환값이 `Result`/`Option`이 아니라 원시 값(`0`, `false` 등)인 경우를 전수 리뷰.
- Runtime: 파일을 다양한 지점에서 truncate한 corpus(마지막 1바이트, 중간, NAL 헤더 직후 등)로 fuzzing해, 파싱 결과가 "정상처럼 보이는" 케이스가 없는지 확인.

**예외**:
- 없음 — bit-reader의 EOF는 항상 명시적으로 구분되어야 한다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-core/src/bitreader.rs`의 `read_bit()`(라인 129-144)이 `if self.byte_offset >= self.data.len() { return Err(BitvueError::UnexpectedEof(self.position())) }`로 EOF를 명시적 `Err`로 반환하며 `0`을 반환하지 않음. 이 구현을 HEVC/AVC/VVC/VP9/MPEG2/AV3/AV1이 공유하고, AVS3 독립 구현(`crates/bitvue-avs3/src/bitreader.rs:37-39`)도 동일하게 `bits_remaining() < n`이면 `Err(Avs3Error::UnexpectedEof)`를 반환. 이 문서가 우려하는 "EOF를 0비트로 취급" 패턴은 확인한 범위에서 발견되지 않음.

---

### PARSE-027: 오류 위치 정보 손실
**분류**: PARSE · **심각도**: Medium · **탐지**: Structural/Manual

**나쁜 예**:
```rust
#[derive(Debug)]
enum ParseError {
    OutOfBounds,
    InvalidValue,
    UnexpectedEof,
    // 어느 필드, 어느 바이트/비트 위치에서 발생했는지 정보가 전혀 없음
}

fn read_ue(reader: &mut BitReader) -> Result<u32, ParseError> {
    // ...
    Err(ParseError::InvalidValue) // "뭐가 잘못됐는지"만 알고 "어디서"는 모름
}
```

**문제**:
- 분석 도구의 핵심 가치 중 하나는 "이 파일의 몇 번째 바이트, 어느 신택스 요소에서 문제가 있는지"를 사용자에게 정확히 알려주는 것인데, 위치 정보 없는 에러는 이 가치를 완전히 잃는다.
- 개발자 입장에서도 "SPS 파싱 중 어디선가 InvalidValue가 났다"는 정보만으로는 재현/디버깅이 거의 불가능하다 — 실제로는 로그에 별도로 위치를 찍는 임시방편이 여기저기 흩어지게 된다.
- 에러 타입이 위치 정보를 구조적으로 갖고 있지 않으면, 나중에 "에러 위치를 hex view에서 하이라이트"하는 기능(분석 도구에서 매우 자연스러운 요구사항)을 추가하려 할 때 에러 타입 자체를 전면 재설계해야 한다.

**발생 조건**:
- 사용자가 "이 파일이 왜 파싱이 안 되나요"라고 물었을 때 답할 수 있는 정보가 로그의 스택트레이스뿐인 상황.
- 여러 신택스 요소가 중첩된 깊은 파싱 경로(SPS 안의 VUI 안의 HRD 파라미터)에서 에러가 발생했을 때, 어느 중첩 레벨인지조차 알 수 없는 경우.

**권장**:
```rust
#[derive(Debug)]
struct ParseError {
    kind: ParseErrorKind,
    byte_offset: u64,       // 파일/버퍼 내 절대 바이트 위치
    bit_offset: u8,         // 바이트 내 비트 오프셋(0..8)
    field_path: FieldPath,  // PARSE-028: 어느 필드였는지 (예: sps.vui.hrd_parameters.cpb_size)
}

#[derive(Debug)]
enum ParseErrorKind {
    OutOfBounds { needed: usize, remaining: usize },
    InvalidValue { expected: String, actual: u64 },
    UnexpectedEof,
}

fn read_ue(reader: &mut BitReader, field_path: &FieldPath) -> Result<u32, ParseError> {
    // ...
    Err(reader.error_at(ParseErrorKind::InvalidValue { .. }, field_path.clone()))
}
```
- 에러 타입에 항상 byte/bit 위치를 구조적 필드로 포함시키고, 이를 `BitReader`가 현재 위치를 기반으로 자동으로 채워주는 헬퍼(`reader.error_at(kind, path)`)로 일관되게 생성한다.
- 이 정보는 그대로 UI의 hex view/신택스 트리에서 "여기서 문제 발생"을 하이라이트하는 데 재사용할 수 있어, 안전성 개선이 곧바로 사용자 경험 개선으로 이어진다.

**탐지 방법**:
- Structural: `ParseError` enum/struct 정의에 위치 필드가 있는지, 모든 생성 지점이 이를 채우는지 리뷰.
- Manual: 실제 malformed 파일을 열었을 때 사용자에게 보여지는 에러 메시지에 "몇 번째 바이트"가 포함되는지 수동 확인.

**예외**:
- 파일 레벨보다 상위(예: "이 파일 확장자가 지원되지 않습니다")의 에러는 바이트 위치 개념 자체가 없으므로 이 필드를 `Option`으로 두거나 별도 에러 카테고리로 분리한다.

**Bitvue 판정**: Suspected (부분 확인, 2026-08-01) — `crates/bitvue-core/src/error.rs`의 `BitvueError::Parse { offset: u64, message: String }`와 `UnexpectedEof(u64)`는 위치 정보(비트 오프셋)를 구조적으로 포함하지만, 코덱별 상위 에러 타입(예: `HevcError::InvalidData(String)`, HEVC SPS의 `MAX_PIC_DIMENSION` 검증 실패 등 다수의 semantic validation 에러)은 `String` 메시지만 담고 별도의 byte/bit offset 필드가 없어 낮은 레벨(비트 리더)과 높은 레벨(의미 검증)의 에러 위치 정보 보존 수준이 일관되지 않음. hex view 하이라이트 등 UI 연동까지 실제로 이어지는지는 미확인.

---

### PARSE-028: field path를 문자열 concatenation으로 생성
**분류**: PARSE · **심각도**: Low · **탐지**: Static/Structural

**나쁜 예**:
```rust
fn parse_hrd_parameters(reader: &mut BitReader, path_prefix: String) -> Result<HrdParameters, ParseError> {
    let cpb_cnt = reader.read_ue()
        .map_err(|e| e.with_context(format!("{}.hrd_parameters.cpb_cnt_minus1", path_prefix)))?;
    // 필드마다 매번 format!()으로 문자열을 새로 조립 — 성공 경로에서도 비용 발생 가능
    Ok(HrdParameters { cpb_cnt })
}
```

**문제**:
- `format!()`으로 매번 경로 문자열을 조립하면, 특히 이 조립이 (에러가 없는) 성공 경로에서도 실행되도록 잘못 배치되면 대량의 임시 `String` 할당이 hot path에서 발생한다.
- 문자열 기반 경로는 오타(`"cpb_cnt_minus1"` vs `"cpb_cnt_minus_1"`)에 컴파일러가 아무 도움을 주지 못하고, 리팩터링(필드명 변경) 시 문자열 리터럴을 모두 수동으로 찾아 바꿔야 한다.
- "어느 필드인지"를 구조화된 데이터(enum/구조체 경로)가 아니라 자유 형식 문자열로 표현하면, PARSE-027에서 언급한 "UI에서 필드 경로로 신택스 트리 노드를 찾아 하이라이트"하는 기능을 구현할 때 문자열 파싱이라는 불필요한 왕복이 필요해진다.

**발생 조건**:
- 에러 컨텍스트를 추가하려는 좋은 의도로 시작했지만, 편의상 `format!("{}.{}", parent, field_name)`을 필드마다 반복하게 되는 경우.
- 깊이 중첩된 신택스 구조(VUI 안의 HRD 안의 sub-layer HRD)에서 경로가 길어질수록 이 문제가 누적된다.

**권장**:
```rust
#[derive(Debug, Clone)]
enum FieldPathSegment {
    Sps, Pps, Vui, HrdParameters, SubLayerHrd(u8), Field(&'static str),
}

#[derive(Debug, Clone, Default)]
struct FieldPath(Vec<FieldPathSegment>);

impl FieldPath {
    fn push(&self, seg: FieldPathSegment) -> FieldPath {
        let mut new_path = self.0.clone();
        new_path.push(seg);
        FieldPath(new_path) // 실패 시에만 최종적으로 문자열화(Display)
    }
}

fn parse_hrd_parameters(reader: &mut BitReader, path: &FieldPath) -> Result<HrdParameters, ParseError> {
    let path = path.push(FieldPathSegment::HrdParameters);
    let cpb_cnt = reader.read_ue()
        .map_err(|e| e.with_path(path.push(FieldPathSegment::Field("cpb_cnt_minus1"))))?;
    Ok(HrdParameters { cpb_cnt })
}
```
- 필드 경로는 문자열이 아니라 `&'static str`/enum variant로 구성된 구조화된 타입(`FieldPath`)으로 표현하고, 사람이 읽을 문자열로의 변환(`Display` 구현)은 실제로 에러를 출력/표시하는 마지막 순간에만 수행한다.
- `&'static str`을 쓰면 필드명 자체는 런타임 할당 없이(리터럴이므로) 표현 가능하고, `FieldPath` 자체의 `Vec` 복제 비용도 에러 경로에서만 발생하므로 성공 경로에는 영향이 없다.

**탐지 방법**:
- Static: `format!(` 호출이 파서 hot path(에러가 아닌 정상 흐름)에 존재하는지 grep 후 개별 검토.
- Structural: 에러 컨텍스트/경로 표현이 문자열 기반인지 구조화 타입 기반인지 아키텍처 리뷰.

**예외**:
- 에러가 실제로 발생한 이후(즉 이미 느린 경로에 들어선 뒤)의 최종 메시지 조립 단계에서는 `format!()`을 자유롭게 써도 성능에 영향이 없다.

**Bitvue 판정**: N/A (기능 부재, 2026-08-01) — `FieldPath`/`field_path`/`with_context(format!(...))` 같은 구조화된 필드 경로 개념 자체를 코드베이스 전체에서 찾지 못함(grep 결과 무관한 매치 1건뿐). 이 항목이 지적하는 "문자열 concatenation으로 필드 경로를 만드는" 안티패턴이 성립하려면 애초에 필드 경로 추적 기능이 존재해야 하는데, 현재는 그 기능 자체가 구현되어 있지 않아 이 특정 안티패턴이 나타날 수조차 없음(PARSE-027의 위치 정보 부재와 연결되는 상위 기능 격차).

---

### PARSE-029: display 정보와 parsing 상태 결합
**분류**: PARSE · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
struct Sps {
    profile_idc: u8,
    level_idc: u8,
    // 파싱 결과 구조체에 UI 표시용 포맷 문자열까지 함께 저장
    display_label: String, // 예: "Main Profile, Level 4.1"
    tree_expanded: bool,   // UI 트리 노드가 펼쳐졌는지 여부까지 파서 상태에 포함
}

fn parse_sps(reader: &mut BitReader) -> Result<Sps, ParseError> {
    let profile_idc = reader.read_u8()?;
    let level_idc = reader.read_u8()?;
    let display_label = format!("{} Profile, Level {}", profile_name(profile_idc), level_idc as f32 / 10.0);
    Ok(Sps { profile_idc, level_idc, display_label, tree_expanded: false })
}
```

**문제**:
- 파서(코덱 크레이트)가 UI 관심사(표시 라벨, 트리 펼침 상태)까지 알고 있으면, 코덱 크레이트가 프론트엔드 표현 방식에 종속되어 재사용성이 떨어진다 — 예를 들어 CLI 도구나 다른 UI 프레임워크에서 같은 파서를 재사용하려 해도 UI 전용 필드가 항상 따라온다.
- `tree_expanded: bool` 같은 순수 UI 상태가 파싱 결과 구조체에 섞이면, "같은 SPS를 다시 파싱했는데 UI 상태가 초기화된다/안 된다"는 혼란스러운 버그의 원인이 된다 — 파싱은 순수해야 하고 UI 상태는 별도 계층에서 관리되어야 한다.
- `display_label`처럼 파생 가능한 표시 문자열을 파싱 시점에 미리 계산해 저장하면, 로케일(다국어) 지원이나 표시 포맷 변경이 필요할 때 파서를 다시 실행해야 하는 불필요한 결합이 생긴다.

**발생 조건**:
- Tauri(Rust) 백엔드가 파싱한 구조체를 그대로 직렬화해 React 프론트엔드로 넘기는 구조에서, "프론트에서 표시하기 편하게" 미리 가공된 필드를 파서 쪽에 추가하고 싶은 유혹이 생길 때.
- 여러 코덱 크레이트가 각자 다른 방식으로 "표시용 필드"를 파싱 결과에 끼워 넣어, 크레이트마다 구조체 설계 일관성이 깨지는 경우.

**권장**:
```rust
// codec crate: 순수 파싱 결과만 담는다
struct Sps {
    profile_idc: u8,
    level_idc: u8,
}

// presentation layer (별도 크레이트 또는 모듈): 파싱 결과로부터 표시용 정보를 파생
fn format_profile_level(sps: &Sps) -> String {
    format!("{} Profile, Level {}", profile_name(sps.profile_idc), sps.level_idc as f32 / 10.0)
}
```
- 코덱 크레이트는 스펙에 정의된 신택스 요소만 담은 순수 데이터 구조를 반환하고, 표시 라벨/포맷팅/UI 상태는 별도의 presentation 레이어(프론트엔드 또는 별도 Rust 모듈)에서 파싱 결과로부터 파생시킨다.
- 이렇게 분리하면 같은 파싱 결과를 여러 표현 방식(사람이 읽는 요약, JSON 덤프, hex 트리)으로 재사용할 수 있고, 로케일/포맷 변경이 파서 재실행 없이 가능해진다.

**탐지 방법**:
- Structural: 코덱 크레이트의 공개 구조체 필드에 `String` 타입의 "label"/"display"류 이름이나 `bool` 타입의 "expanded"/"selected"류 UI 상태 필드가 있는지 grep해 전수 점검.
- Manual: 코덱 크레이트가 UI 프레임워크(Tauri, React 관련 타입)에 의존성을 갖고 있지 않은지 `Cargo.toml` 의존성 감사.

**예외**:
- 파싱 결과 구조체에 `#[derive(Serialize)]`로 프론트엔드 직렬화를 위한 애너테이션을 다는 것 자체는 문제가 아니다(직렬화 가능성은 UI 종속이 아님) — 문제는 "표시를 위해 가공된 값"을 저장하는 것이다.

**Bitvue 판정**: Confirmed (확인, 2026-08-01) — `crates/bitvue-hevc/src/syntax/mod.rs`, `crates/bitvue-vvc/src/syntax/mod.rs`, `crates/bitvue-vp9/src/syntax/mod.rs` 각각에 정의된 `SyntaxNode` 구조체가 `pub value: Option<String>` 필드(hevc/vp9 라인 15, vvc 라인 11)로 UI 트리 표시용으로 포맷된 문자열 값을 파서 결과 구조체에 직접 포함. 이 "시각화용 신택스 트리" 모듈이 코덱마다 독립적으로 중복 구현되어 있어, 이 문서가 우려하는 "파서가 UI 관심사를 알고 있다"는 결합이 구조적으로 존재. 다만 이는 우발적 결합이라기보다 "신택스 트리 탐색기"라는 이 도구의 핵심 기능을 위해 의도적으로 설계된 것으로 보이며, `tree_expanded: bool` 같은 순수 UI 상태 필드는 발견되지 않음(순도 낮은 결합이지 최악의 형태는 아님).

---

### PARSE-030: 모든 syntax를 eager materialization
**분류**: PARSE · **심각도**: Medium · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
fn parse_file(data: &[u8]) -> Result<Vec<Frame>, ParseError> {
    let mut frames = Vec::new();
    for nal in split_nals(data) {
        let frame = parse_frame_fully(nal)?; // 모든 신택스 요소를 즉시 전부 파싱
        frames.push(frame); // 사용자가 절대 보지 않을 프레임까지 전부 메모리에 상주
    }
    Ok(frames)
}
```

**문제**:
- 사용자가 실제로 조사하는 것은 보통 전체 프레임 중 일부(특정 GOP, 특정 프레임 범위)인데, 파일을 여는 순간 모든 프레임의 모든 신택스 요소를 완전히 파싱해 메모리에 올리면 대용량 파일에서 "열기"만으로 수십 초가 걸리고 메모리도 파일 크기의 몇 배를 소모한다.
- Eager materialization은 PARSE-023/024(불필요한 복사)와 결합되면 문제가 배가된다 — 복사 비용과 파싱 비용이 모두 "전체 파일 × 즉시"로 곱해진다.
- 분석 도구의 반응성(사용자가 파일을 열자마자 탐색을 시작할 수 있어야 함) 요구사항과 정면으로 충돌하는 설계다.

**발생 조건**:
- 초기 프로토타입에서 "일단 다 파싱해서 구조체에 담아두면 이후 로직이 단순해진다"는 이유로 채택했다가, 실사용 대용량 파일(수 GB, 수만 프레임)에서 성능 문제로 드러나는 전형적인 패턴.
- 프레임 목록 트리 UI가 있고, 사용자가 스크롤하며 특정 프레임을 펼칠 때만 상세 정보가 필요한 구조인데 백엔드가 이를 구분하지 못하는 경우.

**권장**:
```rust
struct FrameIndex {
    offset: u64,
    size: u32,
    frame_type: FrameType, // 목록 표시에 필요한 최소 정보만 가벼운 스캔으로 확보
}

fn scan_file(data: &[u8]) -> Result<Vec<FrameIndex>, ParseError> {
    // 1단계: 헤더만 가볍게 훑어 인덱스만 구축(전체 신택스 파싱 없음)
    split_nals(data).map(|nal| scan_frame_header(nal)).collect()
}

fn parse_frame_detail(data: &[u8], index: &FrameIndex) -> Result<Frame, ParseError> {
    // 2단계: 사용자가 실제로 펼친 프레임만 온디맨드로 완전 파싱
    let bytes = &data[index.offset as usize..(index.offset + index.size as u64) as usize];
    parse_frame_fully(bytes)
}
```
- "가벼운 인덱싱(1단계)"과 "온디맨드 상세 파싱(2단계)"을 구조적으로 분리한다 — 파일을 여는 즉시 필요한 것은 목록 UI를 그릴 수 있는 최소 정보(오프셋, 크기, 타입)뿐이다.
- 상세 파싱 결과는 LRU 캐시 등으로 최근 조회한 프레임 몇 개만 메모리에 유지하고, 스크롤이 멀어지면 evict해 메모리 사용량을 파일 크기와 무관하게 상한을 둘 수 있다.
- 이 구조는 PARSE-024(zero-copy 참조)와 자연스럽게 결합된다 — 1단계 인덱스는 원본 버퍼에 대한 offset/size만 가지므로 복사가 필요 없다.

**탐지 방법**:
- Runtime: 파일 크기별로 "파일 열기" 소요 시간과 피크 메모리 사용량을 벤치마크해, 파일 크기에 선형 이상으로 비례하는지 확인.
- Structural: 파일을 여는 최초 진입점 함수가 전체 프레임 리스트를 반환하는 타입인지, 인덱스만 반환하는 타입인지 API 시그니처 리뷰.
- Manual: 실제 대용량(수 GB) 테스트 파일로 "열기 → 첫 프레임 표시까지" 체감 지연을 수동 확인.

**예외**:
- 파일 크기가 처음부터 작다고 보장되는 컨텍스트(예: 단일 SPS/PPS만 추출해 보여주는 작은 유틸리티 커맨드)에서는 eager materialization이 오히려 더 단순하고 적절한 선택일 수 있다.

**Bitvue 판정**: Confirmed (확인, 2026-08-01) — `crates/bitvue-hevc/src/nal.rs`, `crates/bitvue-avc/src/nal.rs`, `crates/bitvue-vvc/src/nal.rs`의 `parse_nal_units(data)`가 파일 전체를 스캔해 모든 NAL 유닛에 대해 즉시 헤더 파싱 + payload 복사(`to_vec()`) + emulation prevention 제거를 수행하고 `Vec<NalUnit>`으로 완전히 반환(PARSE-024와 동일 코드 지점). "가벼운 인덱싱(오프셋/크기/타입만)"과 "온디맨드 상세 파싱"을 분리하는 2단계 구조(`FrameIndex`/`scan_file`류)는 이 함수들에서 발견되지 않아, 이 문서의 나쁜 예와 구조적으로 일치.

---

### PARSE-031: 컨테이너 샘플 테이블 누적 offset에 checked_add 미사용
**분류**: PARSE · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
// MP4 stco/co64 + stsz를 조합해 각 샘플의 절대 offset을 누적 계산
fn compute_sample_offsets(chunk_offset: u64, sample_sizes: &[u32]) -> Vec<u64> {
    let mut offsets = Vec::with_capacity(sample_sizes.len());
    let mut cur = chunk_offset;
    for &size in sample_sizes {
        offsets.push(cur);
        cur += size as u64; // 누적 덧셈에 overflow 검증 없음
    }
    offsets
}
```

**문제**:
- PARSE-001은 "단발성 offset+length 검증"을 다뤘지만, 이 항목은 반복 누적되는 offset 계산에서 발생하는 문제로 별개다 — `sample_sizes` 배열의 개별 항목이 조작되면 누적값이 서서히 또는 급격히 wrap될 수 있고, 개별 덧셈은 다 "정상 범위"처럼 보여도 누적 결과가 틀어진다.
- `stsz`(샘플 크기 테이블)의 개별 항목이 malformed되어 매우 큰 값(예: `u32::MAX`)이 섞여 있으면, 몇 번의 누적만으로도 `cur`이 `u64` 범위를 넘을 수 있고(현실적으로는 매우 크지만 불가능하지 않음), 더 흔하게는 이렇게 계산된 offset이 실제 파일 크기를 훨씬 초과해 이후 `read_sample`이 완전히 엉뚱한(혹은 OOB) 위치를 가리키게 된다.
- 이 계산은 파일을 열자마자 전체 샘플에 대해 한 번에 수행되는 경우가 많아(인덱스 구축 단계), 단 하나의 손상된 `stsz` 항목이 이후 모든 샘플의 offset을 연쇄적으로 오염시킨다.

**발생 조건**:
- MP4/MOV의 `stsz`+`stco`/`co64`, MKV의 누적 Cluster 오프셋처럼 "이전 값 + 현재 크기"로 다음 위치를 유도하는 모든 컨테이너 인덱싱 구조.
- 손상되거나 fuzzing된 컨테이너 메타데이터 박스.

**권장**:
```rust
fn compute_sample_offsets(chunk_offset: u64, sample_sizes: &[u32], file_len: u64) -> Result<Vec<u64>, ParseError> {
    let mut offsets = Vec::with_capacity(sample_sizes.len());
    let mut cur = chunk_offset;
    for &size in sample_sizes {
        if cur > file_len {
            return Err(ParseError::SampleOffsetExceedsFileSize { offset: cur, file_len });
        }
        offsets.push(cur);
        cur = cur.checked_add(size as u64).ok_or(ParseError::OffsetOverflow {
            offset: cur as usize, length: size as usize,
        })?;
    }
    Ok(offsets)
}
```
- 누적 계산의 매 스텝마다 `checked_add`를 쓰고, 추가로 "지금까지 계산된 offset이 이미 파일 크기를 넘는지"를 함께 검증해 조기에 실패시킨다(끝까지 계산한 뒤 마지막에야 검증하면 이미 늦다).
- 인덱스 구축 단계에서 이런 검증을 통과하지 못한 샘플은 "인덱싱 불가"로 표시하고, 그 이전까지의 유효한 샘플만이라도 사용자에게 보여주는 관대한 처리(PARSE-019 원칙)를 함께 고려한다.

**탐지 방법**:
- Static: 컨테이너 파서에서 누적 변수(`cur`, `running_offset` 등)에 대한 `+=`를 grep해 `checked_add` 대체 여부 확인.
- Runtime: `stsz` 항목 중 하나를 극단값으로 바꾼 fuzz corpus로 인덱스 구축 단계를 집중 테스트.

**예외**:
- 없음 — 파일 오프셋을 다루는 누적 산술은 항상 checked 연산이어야 한다.

**Bitvue 판정**: Confirmed (확인, 2026-08-01) — `crates/bitvue-formats/src/mp4.rs:237`의 샘플 오프셋 누적 계산 `current_offset += size as u64;`가 checked_add 없이 원시 `+=`를 사용 — 이 문서의 나쁜 예와 정확히 동일한 패턴. 다만 (1) `stsz`의 `sample_count`가 `MAX_ENTRY_COUNT`(1000만)로 상한 검증되고 각 size가 u32이므로 이론상 최대 누적값이 u64 범위에 크게 못 미쳐 실제 overflow는 도달 불가능하고, (2) 이후 실제 샘플 읽기 직전에는 별도로 `offset.checked_add(size)` + `end > data.len()` 검증(라인 276-291)이 다시 수행되어 다운스트림에서 이중으로 방어됨. 패턴 자체는 존재하나 실질 위험도는 낮음.

---

### PARSE-032: profile/level constraint 필드가 해상도와 교차검증되지 않음
**분류**: PARSE · **심각도**: Medium · **탐지**: Semantic/Manual

**나쁜 예**:
```rust
fn parse_sps(reader: &mut BitReader) -> Result<Sps, ParseError> {
    let profile_idc = reader.read_u8()?;
    let level_idc = reader.read_u8()?;
    let pic_width = reader.read_ue()?;
    let pic_height = reader.read_ue()?;
    // profile_idc/level_idc는 읽어서 구조체에 저장만 하고, pic_width/height와의 정합성은 전혀 확인하지 않음
    Ok(Sps { profile_idc, level_idc, pic_width, pic_height })
}
```

**문제**:
- 코덱 스펙은 각 레벨(level)마다 허용되는 최대 해상도×프레임레이트(예: HEVC Level 4.1은 최대 8912896 샘플)를 명시하는데, 이를 검증하지 않으면 "Level 1.0인데 8K 해상도" 같은 스펙 위반 조합을 파서가 그대로 통과시킨다.
- 분석 도구의 핵심 가치 중 하나가 "이 스트림이 표준을 준수하는가"를 보여주는 것인데, 이 교차검증이 없으면 conformance 위반을 탐지하는 기능 자체가 누락된다 — 이는 버그라기보다 "기능 미비"에 가깝지만, 안전성 카탈로그에 포함하는 이유는 이런 불일치가 종종 실제 파싱 버그(잘못된 필드 오프셋)의 증상으로도 나타나기 때문이다.
- 인코더 버그나 스트림 손상으로 이런 불일치가 발생했을 때, 이를 감지하지 못하면 사용자가 "왜 이 스트림이 특정 디코더에서 재생이 안 되는지"를 분석 도구로 파악할 수 없다.

**발생 조건**:
- 스펙 위반 인코더 출력, 트랜스코딩 과정에서 레벨 필드를 갱신하지 않고 해상도만 바꾼 손상된 스트림.
- 의도적으로 조작된 conformance 테스트 벡터(negative test case)를 분석할 때.

**권장**:
```rust
fn validate_level_constraints(sps: &Sps) -> Vec<ParseWarning> {
    let mut warnings = Vec::new();
    if let Some(max_samples) = level_max_luma_samples(sps.profile_idc, sps.level_idc) {
        let actual_samples = sps.pic_width as u64 * sps.pic_height as u64;
        if actual_samples > max_samples {
            warnings.push(ParseWarning::LevelConstraintViolated {
                field: "pic_width x pic_height",
                level_idc: sps.level_idc,
                max_samples,
                actual_samples,
            });
        }
    }
    warnings
}
```
- 파싱 자체는 관대하게 통과시키되(fatal 처리하지 않음, PARSE-019 원칙), 스펙이 정의한 프로파일/레벨별 제약표를 별도 semantic validation 계층으로 구현해 위반 시 사용자에게 경고로 노출한다.
- 이 제약표는 코덱 스펙 부록에 표 형태로 명시되어 있으므로, 상수 테이블로 옮기고 출처(스펙 절 번호)를 주석으로 남겨 유지보수성을 확보한다.

**탐지 방법**:
- Semantic: 코덱별 conformance 테스트 벡터(공식 JCT-VC/AOM 등에서 제공하는 negative test suite)를 회귀 corpus로 활용해 위반 탐지 여부를 검증.
- Manual: 프로파일/레벨 제약 검증 로직이 존재하는지, 최신 스펙 개정판의 표와 일치하는지 주기적 수동 검토.

**예외**:
- 순수 구문 파싱(syntax parsing)만을 목표로 하는 저수준 파서 계층에서는 이 검증을 생략하고, 상위 semantic validation 계층에만 책임을 두는 계층 분리도 합리적이다 — 다만 그 계층이 실제로 존재하고 호출되어야 한다.

**Bitvue 판정**: Confirmed (부재 확인, 2026-08-01) — `level_max_luma_samples`/`LevelConstraint`/`validate_level` 등의 함수나 프로파일·레벨별 해상도 상한 테이블을 HEVC/AVC/VVC/AV1 크레이트 전체에서 grep으로 찾지 못함. 각 SPS 파서가 `profile_idc`/`level_idc`와 `pic_width`/`pic_height`를 각각 읽어 구조체에 저장만 할 뿐, 이 둘을 교차검증해 레벨 제약 위반을 경고하는 semantic validation 계층이 구현되어 있지 않음.

---

### PARSE-033: emulation prevention byte 처리가 버퍼 경계에서 off-by-one
**분류**: PARSE · **심각도**: High · **탐지**: Runtime/Manual

**나쁜 예**:
```rust
fn strip_emulation_prevention(ebsp: &[u8]) -> Vec<u8> {
    let mut rbsp = Vec::with_capacity(ebsp.len());
    let mut i = 0;
    while i < ebsp.len() {
        // 0x00 0x00 0x03 패턴을 찾을 때 뒤쪽 바이트를 미리 조회
        if i + 2 < ebsp.len() && ebsp[i] == 0 && ebsp[i + 1] == 0 && ebsp[i + 2] == 0x03 {
            rbsp.push(0);
            rbsp.push(0);
            i += 3; // 0x03 바이트를 건너뜀
        } else {
            rbsp.push(ebsp[i]);
            i += 1;
        }
    }
    rbsp
}
```

**문제**:
- `i + 2 < ebsp.len()` 조건은 `i + 2 == ebsp.len() - 1`(즉 `i+2`가 마지막 유효 인덱스)인 경계에서 `<` 대신 `<=`가 맞는지 헷갈리기 쉬운 전형적인 off-by-one 소스다 — 이 특정 코드는 실제로는 안전하지만(`i+2 < len`이면 `i+2`는 유효 인덱스), 유사한 변형에서 `<=`를 잘못 써서 `ebsp[i+2]`가 범위를 넘는 경우가 실제로 자주 발생한다.
- NAL 페이로드의 마지막 3바이트가 정확히 `00 00 03`인 경우(스펙상 유효한 패턴)와, 마지막 2바이트가 `00 00`으로 끝나는 경우(이 뒤에 emulation prevention byte가 없어도 되는 경우 — 스펙은 NAL의 마지막 바이트가 0이 아니어야 한다고 규정하므로 실제로는 이런 케이스 자체가 드물지만, malformed 입력은 이를 위반할 수 있다)를 각각 정확히 처리해야 한다.
- 이 함수는 코덱마다(AVC/HEVC/VVC) 거의 동일한 로직이 독립적으로 재구현되는데, 하나의 크레이트에서 경계 처리를 고쳐도 다른 크레이트에는 반영되지 않아 같은 버그가 여러 곳에 중복 존재할 수 있다.

**발생 조건**:
- NAL 페이로드가 정확히 `... 00 00 03`으로 끝나는 경계 케이스, 혹은 `00 00`으로 끝나 다음 바이트 조회가 필요한데 버퍼가 거기서 끝나는 케이스.
- fuzzing이 NAL 페이로드의 마지막 몇 바이트만 집중적으로 변형하는 경우 이런 경계 버그를 잘 찾아낸다.

**권장**:
```rust
fn strip_emulation_prevention(ebsp: &[u8]) -> Vec<u8> {
    let mut rbsp = Vec::with_capacity(ebsp.len());
    let mut zero_run = 0u8;
    for &byte in ebsp {
        // 인덱스 미리보기 대신, 이미 처리한 바이트의 연속된 0 개수만 추적 — 경계 조건이 원천적으로 사라짐
        if zero_run >= 2 && byte == 0x03 {
            zero_run = 0;
            continue; // emulation prevention byte는 출력하지 않음
        }
        rbsp.push(byte);
        zero_run = if byte == 0 { zero_run + 1 } else { 0 };
    }
    rbsp
}
```
- "다음 바이트를 미리 조회하는" 방식 대신, "지금까지 본 연속 0의 개수"를 상태로 추적하며 한 바이트씩 순전진하는 방식(PARSE-023 예시와 동일한 패턴)으로 바꾸면 인덱스 산술 자체가 사라져 off-by-one의 여지가 원천적으로 없어진다.
- 이 로직을 코덱별로 재구현하지 말고, 공통 유틸리티 크레이트에 한 번만 구현해 AVC/HEVC/VVC가 공유하게 한다(세 표준 모두 emulation prevention byte 규칙이 동일).

**탐지 방법**:
- Runtime: NAL 페이로드의 마지막 1~4바이트를 `00 00 03`, `00 00`, `00 00 00`, `03`으로 각각 설정한 경계값 unit 테스트.
- Structural: `i + N < len`/`i + N <= len` 형태의 미리보기 인덱싱이 여러 코덱 크레이트에 중복 구현되어 있는지 grep으로 전수 조사 후 공통화 검토.

**예외**:
- 없음 — 이 변환은 모든 코덱에서 스펙이 동일하게 규정하므로 공통 구현으로 통일하지 않을 이유가 없다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-core/src/bitreader.rs`의 `remove_emulation_prevention_bytes()`(라인 790-818)가 HEVC/AVC/VVC 세 크레이트 모두에서 `crate::bitreader::remove_emulation_prevention_bytes`로 import되어 공유되는 단일 구현(이 문서가 우려하는 "코덱마다 독립 재구현" 패턴이 아님). 경계 조건도 `if i + 2 < data.len() && ...`로 `data[i+2]`가 항상 유효 인덱스임을 보장(off-by-one 없음) — 이 문서의 나쁜 예 코드가 스스로 인정하듯 "이 특정 코드는 실제로는 안전한" 형태와 일치.

---

### PARSE-034: 코덱별 bit reader가 MSB-first/LSB-first 기본값을 암묵적으로 다르게 채택
**분류**: PARSE · **심각도**: High · **탐지**: Structural/Manual

**나쁜 예**:
```rust
// hevc 크레이트의 BitReader: MSB-first (스펙 규정대로 구현)
impl<'a> hevc::BitReader<'a> {
    fn read_bit(&mut self) -> u8 {
        let byte = self.data[self.bit_pos / 8];
        (byte >> (7 - self.bit_pos % 8)) & 1 // MSB부터
    }
}

// vp9 크레이트의 BitReader: 별도 개발자가 독립적으로 구현하며 LSB-first로 착각해 구현
impl<'a> vp9::BitReader<'a> {
    fn read_bit(&mut self) -> u8 {
        let byte = self.data[self.bit_pos / 8];
        (byte >> (self.bit_pos % 8)) & 1 // LSB부터 — VP9도 실제로는 MSB-first인데 실수로 반대로 구현
    }
}
```

**문제**:
- 이 프로젝트 구조상 코덱마다 독립된 크레이트에 독립된 bit-reader가 존재하는데("각 코덱이 자기 크레이트를 가진다"는 전제), 이 구현들이 서로 다른 개발자/시점에 작성되면 MSB-first/LSB-first 관례가 은연중에 갈릴 위험이 실질적으로 존재한다.
- 실제로 대부분의 비디오 코덱 표준(AVC/HEVC/VVC/AV1/VP9)은 빅엔디안 비트 순서(MSB-first)를 사용하지만, "일반적인 프로그래밍에서 비트 연산은 LSB-first가 더 익숙하다"는 개발자의 직관과 충돌해 실수가 나올 수 있다.
- 이 버그는 파싱이 "완전히 실패"하지 않고 "그럴듯하지만 미묘하게 틀린" 값을 낳는 경우가 많아(비트가 뒤집혀도 일부 필드는 우연히 그럴듯한 범위의 값이 나올 수 있음) 조기에 발견되지 않고, 특정 필드 조합에서만 이상 증상이 나타나 디버깅이 매우 어렵다.

**발생 조건**:
- 새 코덱 크레이트를 추가할 때(예: VVC, AV3 지원 추가), 기존 크레이트의 bit-reader를 참고하지 않고 처음부터 새로 구현하는 경우.
- 코덱 크레이트 간 코드 재사용 없이 각자 팀/시점에 독립적으로 작성된 레거시 구현이 합쳐질 때.

**권장**:
```rust
// 모든 코덱 크레이트가 공유하는 공통 유틸리티 크레이트(bitstream-common 등)에
// 단일 구현으로 MSB-first 비트 순서를 강제
pub struct BitReader<'a> { data: &'a [u8], bit_pos: usize }

impl<'a> BitReader<'a> {
    /// 모든 지원 코덱(AVC/HEVC/VVC/AV1/VP9/MPEG-2)은 MSB-first 비트 순서를 사용한다.
    /// 이 구현을 모든 코덱 크레이트가 공유해 순서 불일치를 원천 차단한다.
    pub fn read_bit(&mut self) -> Result<u8, ParseError> {
        // ...
        todo!()
    }
}
```
- 가능하면 모든 코덱 크레이트가 하나의 공통 `bitstream-common` 크레이트의 `BitReader`를 재사용하게 해, 비트 순서 같은 근본 관례를 한 곳에서만 결정하고 검증한다(코덱별로 신택스 파싱 로직은 다르지만 저수준 비트/바이트 접근은 동일해야 함).
- 부득이하게 크레이트별로 독립 구현이 필요하다면(예: 성능 특화), 모든 구현에 대해 "알려진 테스트 벡터에서 동일한 바이트열을 동일한 정수로 디코딩하는가"를 검증하는 공유 conformance 테스트 스위트를 강제한다.

**탐지 방법**:
- Structural: 각 코덱 크레이트의 `read_bit`/`read_bits` 구현을 나란히 놓고 비트 순서 관례가 일치하는지 코드 리뷰로 대조.
- Runtime: 코덱별로 "알려진 바이트열 → 알려진 정수값" 골든 테스트 벡터를 공유 테스트 스위트로 만들어 CI에서 전체 크레이트에 동일하게 실행.
- Manual: 새 코덱 크레이트 추가 시 온보딩 체크리스트에 "비트 순서 conformance 테스트 통과"를 필수 항목으로 명시.

**예외**:
- 컨테이너 포맷 중 일부(예: 특정 필드가 리틀엔디안 바이트 순서를 쓰는 MKV의 일부 정수 필드)는 비트스트림 자체와 다른 관례를 가질 수 있다 — 이는 "예외"가 아니라 애초에 다른 관례이므로, 코덱 비트리더와 컨테이너 바이트리더를 애초에 별개 유틸리티로 명확히 분리해야 한다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — HEVC/AVC/VVC/VP9/MPEG2/AV3/AV1 7개 코덱 크레이트의 `bitreader.rs`는 전부 `bitvue_core::BitReader`(MSB-first, 라인 58 주석 명시)를 `self.inner`로 감싸는 얇은 wrapper로, 비트 순서 관례가 단일 지점에서 결정되어 크레이트 간 불일치가 구조적으로 차단됨. 다만 AVS3(`crates/bitvue-avs3/src/bitreader.rs`)는 이 공유 구현을 쓰지 않고 독립적으로 재구현(파일 최상단 주석 "Simple MSB-first bit reader")했으며, 확인 결과 그 자체는 올바르게 MSB-first로 구현되어 있어 이 항목이 우려하는 구체적 버그(LSB/MSB 불일치)는 발생하지 않았지만, "새 코덱 추가 시 독립 재구현"이라는 이 항목의 발생 조건 자체는 AVS3에서 실제로 일어났음.

---

### PARSE-035: 컨테이너 선언 해상도와 비트스트림 선언 해상도 불일치 미조정
**분류**: PARSE · **심각도**: Medium · **탐지**: Semantic/Runtime

**나쁜 예**:
```rust
struct VideoInfo {
    container_width: u32,  // MP4 tkhd/stsd에서 읽은 값
    container_height: u32,
    codec_width: u32,      // SPS에서 읽은 값
    codec_height: u32,
}

fn build_video_info(tkhd: &Tkhd, sps: &Sps) -> VideoInfo {
    // 두 소스가 다를 수 있다는 사실을 인지하지 못하고 그냥 하나만 골라 쓰거나 나란히 저장만 함
    VideoInfo {
        container_width: tkhd.width,
        container_height: tkhd.height,
        codec_width: sps.pic_width,
        codec_height: sps.pic_height,
    }
}

fn display_resolution(info: &VideoInfo) -> (u32, u32) {
    (info.container_width, info.container_height) // 어느 쪽이 "진짜"인지 판단 로직 없음
}
```

**문제**:
- MP4 `tkhd`의 width/height와 실제 SPS의 `pic_width_in_luma_samples`/cropping 정보는 인코딩 파이프라인의 버그, 트랜스코딩 과정의 메타데이터 미갱신, 혹은 의도적 조작으로 서로 다를 수 있는데, 이를 조정(reconcile)하지 않고 아무 소스나 골라 쓰면 사용자에게 잘못된 해상도가 표시된다.
- 더 심각하게는, 이 두 값 중 하나(대개 컨테이너 값)를 신뢰해 버퍼를 할당한 뒤 실제로는 더 큰 코덱 해상도의 픽셀 데이터를 그 버퍼에 채우려 하면 PARSE-015와 결합해 버퍼 오버런으로 이어질 수 있다.
- 두 값이 다르다는 사실 자체가 "이 파일이 이상하다"는 유용한 진단 정보인데, 이를 그냥 하나로 뭉개버리면 분석 도구로서 놓치지 말아야 할 신호를 놓치는 것이다.

**발생 조건**:
- 트랜스코딩/리먹싱 도구가 컨테이너 메타데이터는 갱신하지 않고 비트스트림만 교체한 파일.
- Cropping window(HEVC `conformance_window`, AV1 `render_width/height` vs `frame_width/height`)까지 고려하면 "해상도"라는 개념 자체가 소스마다 3~4개로 갈라질 수 있는데, 그중 일부만 반영하고 나머지를 무시하는 구현.

**권장**:
```rust
struct ResolutionInfo {
    container: Option<(u32, u32)>,
    codec_coded: (u32, u32),        // SPS의 원본 코딩 해상도
    codec_display: (u32, u32),      // cropping/conformance window 적용 후 실제 표시 해상도
}

fn reconcile_resolution(container: Option<(u32, u32)>, codec_coded: (u32, u32), codec_display: (u32, u32)) -> ResolutionInfo {
    let info = ResolutionInfo { container, codec_coded, codec_display };
    if let Some(c) = container {
        if c != codec_display {
            // 불일치 자체를 warning으로 기록 — 어느 쪽이 옳은지 임의로 결정하지 않는다
            log_warning(ParseWarning::ResolutionMismatch { container: c, codec_display });
        }
    }
    info
}
```
- 컨테이너 선언 해상도, 코덱 coded 해상도, 코덱 display(cropped) 해상도를 각각 별도 필드로 보존하고, 실제로 버퍼를 할당하거나 렌더링할 때는 "이 중 어느 것을 신뢰 기준으로 삼을지"를 명시적 정책(대개는 코덱 display 해상도가 실제 디코딩·렌더링 기준)으로 문서화한다.
- 불일치가 감지되면 사용자에게 표시해, "컨테이너는 1920x1080이라고 하는데 실제 비트스트림은 1280x720입니다" 같은 진단 정보를 분석 결과의 일부로 노출한다.

**탐지 방법**:
- Semantic: 컨테이너 파서와 코덱 파서가 각각 산출한 해상도 값을 비교하는 통합 테스트(리먹싱된 테스트 파일로 의도적 불일치 케이스 포함).
- Manual: cropping/conformance window 관련 필드가 각 코덱 크레이트에서 실제로 파싱되고 있는지, 아니면 coded 해상도만 사용되고 있는지 코드 감사.

**예외**:
- 컨테이너 메타데이터가 아예 없는 raw 비트스트림(Annex B `.264`/`.265`, IVF 등)을 직접 여는 경로에서는 이 항목이 자연히 해당하지 않는다 — 코덱 선언 값만이 유일한 소스가 된다.

**Bitvue 판정**: Confirmed (부재 확인, 2026-08-01) — `ResolutionMismatch`/`reconcile_resolution`/컨테이너 해상도와 코덱 해상도를 비교하는 로직을 `crates/bitvue-formats/src/mp4.rs`와 `src-tauri/src/commands` 전체에서 grep으로 찾지 못함. `tkhd`(컨테이너 width/height)와 SPS의 coded/display 해상도를 각각 별도로 파싱은 하지만 이 둘을 대조해 불일치를 경고하는 코드는 확인되지 않음.

---

### PARSE-036: length 필드가 남은 파일 크기보다 큰데 그대로 신뢰
**분류**: PARSE · **심각도**: Critical · **탐지**: Static/Runtime

**나쁜 예**:
```rust
fn read_box(reader: &mut ByteReader) -> Result<Box, ParseError> {
    let size = reader.read_u32()?; // ISOBMFF box size
    let box_type = reader.read_fourcc()?;
    let payload = reader.read_bytes((size - 8) as usize)?; // size가 남은 파일 크기보다 커도 그대로 요청
    Ok(Box { box_type, payload })
}
```

**문제**:
- ISOBMFF(MP4/MOV) box, MKV Element, AVI chunk 등 거의 모든 컨테이너 포맷은 "이 구조체는 N바이트다"라는 길이 필드를 자체적으로 갖는데, 이 필드가 파일에 실제로 남아있는 바이트 수보다 크게 조작되어도 `read_bytes`가 그 값을 그대로 신뢰해 요청하면 결국 PARSE-001/PARSE-014와 동일한 부류의 문제(OOB 또는 과도한 할당)로 귀결된다.
- 이 항목을 별도로 두는 이유는, "개별 필드의 길이"(PARSE-001)나 "개수 필드"(PARSE-014)와 달리 이건 파서 진입점 그 자체(컨테이너 최상위 box/element 순회)에서 반복적으로 발생하는 패턴이라 한 번의 검증 유틸리티로 프로젝트 전체 컨테이너 파싱 경로를 방어할 수 있는 지점이기 때문이다.
- 특히 `size == 0`(박스가 파일 끝까지 확장됨을 의미하는 특수값)이나 `size == 1`(다음 8바이트가 64비트 확장 크기)같은 ISOBMFF의 특수 인코딩을 놓치고 일반 케이스처럼 처리하면, 이런 특수값 자체가 `size - 8` 같은 계산에서 언더플로우를 일으킬 수도 있다.

**발생 조건**:
- 파일이 다운로드 중 잘렸거나, 스트리밍 도중 마지막 box가 불완전하게 기록된 경우.
- fuzzing이 box/element 크기 필드만 골라 파일 실제 크기보다 크게 조작한 경우 — 컨테이너 최상위 파서는 공격 표면이 가장 넓은 진입점이라 이런 공격의 우선 표적이 된다.

**권장**:
```rust
fn read_box(reader: &mut ByteReader) -> Result<Box, ParseError> {
    let size = reader.read_u32()?;
    let box_type = reader.read_fourcc()?;

    let declared_size = match size {
        0 => reader.remaining_bytes(), // 파일 끝까지 확장되는 특수값
        1 => {
            let size64 = reader.read_u64()?; // 64비트 확장 크기
            size64.checked_sub(16).ok_or(ParseError::InvalidBoxSize { size: size64 })?
        }
        n => (n as u64).checked_sub(8).ok_or(ParseError::InvalidBoxSize { size: n as u64 })?,
    };

    if declared_size > reader.remaining_bytes() {
        return Err(ParseError::LengthExceedsRemainingFile {
            declared: declared_size,
            remaining: reader.remaining_bytes(),
        });
    }
    let payload = reader.read_bytes(declared_size as usize)?;
    Ok(Box { box_type, payload })
}
```
- 컨테이너 파서의 최상위 루프(모든 box/element 읽기의 공통 진입점)에서 "선언된 크기 vs 실제 남은 바이트 수"를 항상 비교하는 검증을 단 한 곳에 구현해 강제한다 — 이 지점이 뚫리면 이후 모든 하위 파싱이 잘못된 경계 위에서 진행되므로, 방어의 우선순위가 가장 높다.
- 크기 필드의 특수값(0, 1 등 포맷별 sentinel)을 문서화하고 각각 명시적으로 처리하며, 뺄셈은 항상 `checked_sub`로 언더플로우를 방지한다.
- 크기가 남은 파일보다 작지만 이 box 안에 중첩된 하위 box들의 크기 합이 부모 box 크기를 넘는 경우도 재귀적으로 동일한 검증을 적용한다(PARSE-025의 substream boundary 원칙과 동일선상).

**탐지 방법**:
- Static: 컨테이너 파서의 모든 "크기 필드 읽기 → 바로 그만큼 read" 지점을 grep해 남은 바이트 수 대조 검증 존재 여부 확인.
- Runtime: 파일 끝 근처의 box 크기 필드를 파일 실제 크기보다 크게 조작한 fuzz corpus, 그리고 파일을 다양한 지점에서 truncate한 corpus를 필수 회귀 세트로 유지.

**예외**:
- 없음 — 컨테이너 최상위 파싱 진입점의 길이 검증은 예외 없이 항상 적용되어야 한다(이 프로젝트가 지원하는 모든 컨테이너: MP4/MOV, MKV/WebM, IVF, AVI 등 공통 원칙).

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-formats/src/mp4.rs`의 최상위 박스 순회 루프(라인 374-415)가 `box_end = box_start.checked_add(header.data_size())` 후 `if box_end > data.len() as u64 { return Err(...) }`(라인 383-389)로 이 문서의 "권장" 코드와 동일한 검증을 수행하며, `BoxHeader::parse()`(라인 80-110)도 `size < header_size`를 별도 검증해 0/1 sentinel 값 처리 시 언더플로우를 방지. `crates/bitvue-av1-codec/src/ivf.rs`도 `frame_end = offset.checked_add(frame_size)` + `frame_end > data.len()` 검증과 `IVF_MAX_FRAME_SIZE` 상한을 함께 적용(라인 261-300).

---

### PARSE-037: VLC/CABAC 테이블을 검증되지 않은 인덱스로 접근
**분류**: PARSE · **심각도**: Critical · **탐지**: Static/Runtime

**나쁜 예**:
```rust
// CABAC 컨텍스트 초기화 테이블 (HEVC 스펙 Table 9-x)
static CTX_INIT_TABLE: [[u8; 2]; 64] = [/* ... 64개 컨텍스트 초기값 ... */];

fn init_context(ctx_idx: usize, slice_qp: u8) -> ContextModel {
    let init_value = CTX_INIT_TABLE[ctx_idx]; // ctx_idx가 파싱 도중 계산된 값이면 검증 없이 인덱싱
    ContextModel::from_init(init_value, slice_qp)
}

fn decode_coeff_level(reader: &mut CabacReader, code_num: usize) -> i32 {
    // VLC 테이블도 동일 패턴: code_num이 스트림에서 유도된 값
    static LEVEL_TABLE: [i32; 32] = [/* ... */];
    LEVEL_TABLE[code_num] // code_num >= 32이면 OOB panic
}
```

**문제**:
- CABAC 컨텍스트 인덱스나 VLC 테이블 인덱스는 신택스 요소 조합(슬라이스 타입, 이전에 디코딩된 신택스 요소 값 등)으로 "계산되어" 나오는 경우가 많은데, 이 계산 로직에 버그가 있거나 입력이 malformed되어 계산 결과가 테이블 크기를 벗어나면 즉시 OOB panic으로 이어진다.
- 이 패턴은 PARSE-007(slice indexing)과 근본 원인은 같지만, 테이블이 코덱 스펙의 고정 상수 배열이라는 점에서 "테이블 자체는 신뢰할 수 있지만 인덱스가 신뢰할 수 없다"는 점을 명확히 짚을 가치가 있다 — 개발자가 "이건 우리가 정의한 상수 테이블이니 안전하다"고 착각하기 쉬운 지점이다.
- CABAC/엔트로피 디코딩 경로는 프레임당 수천~수만 번 호출되는 hot path이기 때문에, 검증을 추가할 때 성능 영향을 고려해야 하며, 이것이 검증을 생략하고 싶은 유혹으로 이어지기 쉽다.

**발생 조건**:
- malformed 스트림이 신택스 요소 값을 조작해 컨텍스트 인덱스 계산식(예: `ctx_idx = base + offset1 + offset2`)의 결과가 테이블 크기를 넘게 만드는 경우.
- 코덱 구현 초기 단계에서 스펙의 인덱스 계산 공식을 잘못 옮겨, 정상 입력에서도 경계값 근처에서 잘못된 인덱스가 나오는 구현 버그.

**권장**:
```rust
fn init_context(ctx_idx: usize, slice_qp: u8) -> Result<ContextModel, ParseError> {
    let init_value = CTX_INIT_TABLE
        .get(ctx_idx)
        .ok_or(ParseError::ContextIndexOutOfRange { ctx_idx, table_size: CTX_INIT_TABLE.len() })?;
    Ok(ContextModel::from_init(*init_value, slice_qp))
}

fn decode_coeff_level(reader: &mut CabacReader, code_num: usize) -> Result<i32, ParseError> {
    static LEVEL_TABLE: [i32; 32] = [/* ... */];
    LEVEL_TABLE
        .get(code_num)
        .copied()
        .ok_or(ParseError::VlcTableIndexOutOfRange { code_num, table_size: LEVEL_TABLE.len() })
}
```
- 정확성이 중요한 경로에서는 `.get()` 기반 검증을 기본으로 하고, 프로파일링으로 실제 hot path임이 확인된 후에만 "이 지점의 인덱스는 상위에서 이미 마스킹/모듈로 연산으로 범위가 보장된다"는 불변식을 `debug_assert!` + 문서화된 이유와 함께 최적화를 고려한다(무증거 최적화 금지).
- 스펙의 인덱스 계산 공식(특히 컨텍스트 인덱스 유도 공식)은 스펙 절 번호를 주석으로 남겨 구현이 정확한지 추후 검증 가능하게 한다.

**탐지 방법**:
- Static: 코덱 크레이트 내 `static`/`const` 배열에 대한 인덱싱을 grep해 `.get()` vs `[]` 사용 비율 감사, `#![deny(clippy::indexing_slicing)]` 적용 범위에 CABAC/VLC 모듈 포함 여부 확인.
- Runtime: CABAC 디코딩 경로를 대상으로 한 전용 fuzz target(엔트로피 코딩된 슬라이스 데이터만 변형)으로 인덱스 계산 버그를 집중 탐색.
- Semantic: 공식 conformance 비트스트림(모든 컨텍스트/VLC 코드가 최소 한 번씩 사용되도록 설계된 스트림)으로 커버리지 확인.

**예외**:
- hot path에서 인덱스가 비트 마스크 연산(`idx & 0x1F`)으로 구조적으로 테이블 크기 내로 보장되는 경우, 매번 `.get()`을 쓰는 대신 그 사실을 명시하는 `debug_assert!`와 주석으로 대체할 수 있다 — 단, 마스크 연산 자체가 스펙과 일치하는지는 여전히 검증되어야 한다.

**Bitvue 판정**: Suspected (부분 확인, 2026-08-01) — `crates/bitvue-vvc/src/overlay_extraction.rs`의 `skip_ctx[skip_ctx_idx]`(라인 890)는 `skip_ctx_idx = left_skip as usize + above_skip as usize`로 항상 0..=2 범위임이 지역적으로 보장되어 raw `[]` 인덱싱이지만 구조적으로 안전(단, `.get()`/`debug_assert!` 명시는 없음). AV1의 `symbol/cdf.rs`에서는 비트스트림에서 유도된 값으로 고정 테이블을 직접 인덱싱하는 위험 패턴을 찾지 못함. 다만 VVC/AV1 외 나머지 코덱(HEVC/AVC 등)의 CABAC/VLC 관련 코드는 깊이 조사하지 못했고, 확인한 두 크레이트도 전수 검토는 아니라 Suspected로 남김.

---

### PARSE-038: NAL start code 스캔이 버퍼 경계를 고려하지 않음
**분류**: PARSE · **심각도**: High · **탐지**: Static/Runtime

**나쁜 예**:
```rust
fn find_next_start_code(data: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 3 <= data.len() {
        if data[i] == 0 && data[i + 1] == 0 && (data[i + 2] == 1 || (data[i + 2] == 0 && data.get(i + 3) == Some(&1))) {
            return Some(i);
        }
        i += 1; // 한 바이트씩만 전진 — 성능은 별개로, 경계 조건이 여러 갈래로 흩어져 있어 실수하기 쉬움
    }
    None
}
```

**문제**:
- start code 탐지 로직이 `00 00 01`(3바이트)과 `00 00 00 01`(4바이트) 두 가지를 한 함수에서 처리하면서 조건식이 복잡해지고, `data.get(i + 3)`처럼 일부만 안전한 접근과 `data[i + 1]`처럼 unsafe한 접근이 뒤섞이면 리뷰어가 전체 경계 안전성을 한눈에 판단하기 어렵다.
- 버퍼의 마지막 근처(`data.len() - 3` ~ `data.len()`)에서 이 스캔이 정확히 동작하는지는 경계 케이스 전용 테스트 없이는 확신하기 어렵다 — 예를 들어 파일이 정확히 `00 00`으로 끝나는 경우 이 구현이 어떻게 반응하는지 코드만 보고는 즉시 알기 어렵다.
- start code 스캔은 파일 전체를 순회하는 최초 진입점(NAL 분리)이므로, 여기서 경계를 잘못 처리하면 마지막 NAL 하나가 누락되거나, 혹은 반대로 존재하지 않는 NAL 경계를 만들어내 이후 파싱 전체가 어긋난다.

**발생 조건**:
- Annex B 형식(start code 기반)의 raw `.264`/`.265`/`.266` 파일에서 마지막 NAL 근처, 또는 파일이 NAL 페이로드 중간에서 잘린 경우.
- NAL 페이로드 내부에 우연히(혹은 emulation prevention byte 처리 누락으로) `00 00 01`과 유사한 바이트열이 등장해 false positive start code로 오인되는 경우(이는 emulation prevention이 코드화 계층에서 이런 시퀀스를 원천적으로 막기 위해 존재하는 이유이기도 하다 — 즉 EBSP 생성 규칙을 신뢰하되, malformed 스트림에서는 이 보장이 깨질 수 있음을 인지해야 한다).

**권장**:
```rust
fn find_next_start_code(data: &[u8], start: usize) -> Option<StartCode> {
    // data.windows()로 경계 산술을 표준 라이브러리에 위임 — 수동 인덱스 계산 제거
    data.get(start..)?
        .windows(3)
        .enumerate()
        .find(|(_, w)| w == [0, 0, 1])
        .map(|(offset, _)| StartCode {
            position: start + offset,
            // 4바이트 start code(00 00 00 01)인지 앞 바이트를 확인
            is_long: start + offset > 0 && data[start + offset - 1] == 0,
        })
}
```
- 가능하면 수동 인덱스 산술 대신 `slice::windows()`, `slice::find()` 같은 표준 라이브러리 이터레이터를 활용해, 경계 조건을 언어/표준 라이브러리가 대신 보장하게 한다 — 직접 짠 반복문보다 검증된 원시 연산에 의존할 때 off-by-one 여지가 줄어든다.
- 이 스캔 로직도 PARSE-034와 마찬가지로 AVC/HEVC/VVC가 공유하는 공통 유틸리티로 구현해, 코덱마다 재구현되며 각기 다른 미묘한 버그를 갖는 것을 방지한다.

**탐지 방법**:
- Runtime: 파일이 `00`, `00 00`, `00 00 00`, `00 00 01`의 각 접두사로 끝나는 경계 케이스 unit 테스트, 그리고 NAL 페이로드 내부에 우연히 `00 00 01`이 등장하는(emulation prevention이 적용되지 않은 malformed) 케이스.
- Structural: 여러 코덱 크레이트에 start code 스캔 로직이 중복 구현되어 있는지 grep으로 조사 후 공통화 검토.

**예외**:
- OBU/박스처럼 자체 길이 필드로 경계가 명시되는 포맷(AV1, ISOBMFF)에는 start code 스캔 자체가 필요 없다 — 이 항목은 Annex B 계열(AVC/HEVC/VVC raw 스트림) 및 유사 포맷에 한정된다.

**Bitvue 판정**: N/A (확인, 2026-08-01) — `crates/bitvue-hevc/src/nal.rs`의 `find_nal_units()`(라인 359-417)가 매 인덱싱 전에 `i + 2 < data.len()`, `i + 3 < data.len()`, `j + 2 < max_scan` 등 경계 조건을 명시적으로 검증하고, 루프 종료 후에도 "Defense in depth" 주석과 함께 `nal_start >= data.len() || nal_end > data.len() || nal_end <= nal_start` 최종 재검증(라인 399-410)까지 수행. 추가로 `MAX_SCAN_DISTANCE`(100MB) 상한으로 O(n²) DoS까지 방지(라인 361-362, 이 카탈로그가 명시하지 않은 보너스 방어). 표준 라이브러리 `windows()` 대신 수동 인덱스 루프를 쓰지만 경계 안전성 자체는 확인됨.

---

### PARSE-039: bit depth / chroma format 조합이 교차검증되지 않음
**분류**: PARSE · **심각도**: Medium · **탐지**: Semantic/Runtime

**나쁜 예**:
```rust
fn parse_sps_range_extension(reader: &mut BitReader, sps: &mut Sps) -> Result<(), ParseError> {
    sps.bit_depth_luma = reader.read_ue()? as u8 + 8;
    sps.bit_depth_chroma = reader.read_ue()? as u8 + 8;
    // chroma_format_idc가 monochrome(0)인데 bit_depth_chroma를 별도로 쓰는 조합,
    // 혹은 bit_depth가 이 프로파일에서 지원되지 않는 값(예: Main profile인데 12bit)인 경우를 검증하지 않음
    Ok(())
}

fn allocate_pixel_buffer(sps: &Sps) -> Vec<u16> {
    let bytes_per_sample = if sps.bit_depth_luma > 8 { 2 } else { 1 };
    // chroma_format_idc를 고려하지 않고 항상 4:2:0으로 가정해 버퍼 크기 계산
    vec![0u16; (sps.pic_width * sps.pic_height * 3 / 2) as usize]
}
```

**문제**:
- `bit_depth_luma`/`bit_depth_chroma`와 `chroma_format_idc`(4:2:0/4:2:2/4:4:4/monochrome)는 서로 조합에 따라 실제 픽셀 버퍼 크기와 메모리 레이아웃이 완전히 달라지는데, 이를 교차검증하지 않고 "항상 4:2:0을 가정"하는 하드코딩(`* 3 / 2`)이 남아있으면, 4:4:4나 monochrome 스트림에서 계산된 버퍼 크기가 실제 필요한 크기보다 작아 버퍼 오버런으로 이어진다.
- 프로파일별로 지원되는 bit depth 범위가 다른데(Main profile은 8bit만, Main 10은 8/10bit, Format Range Extension 프로파일은 최대 16bit 등) 이 제약을 검증하지 않으면 "Main profile인데 16bit"처럼 프로파일 위반이지만 파서는 아무 문제 없이 통과시키는 스트림을 걸러내지 못한다.
- 이 조합 문제는 코덱을 새로 추가할 때(예: VVC의 확장된 chroma format 지원) 기존 코덱에서 검증되던 가정(예: "chroma_format_idc는 항상 1")이 깨지면서 재발하기 쉽다.

**발생 조건**:
- 4:4:4 또는 monochrome(4:0:0) 소스, 10/12bit HDR 소스처럼 "일반적인" 8bit 4:2:0 가정을 벗어나는 스트림을 분석할 때.
- 여러 코덱을 동일한 공통 픽셀 버퍼 할당 유틸리티로 처리하려다, 그 유틸리티가 가장 흔한 조합(8bit 4:2:0)만 가정하고 작성된 경우.

**권장**:
```rust
fn compute_pixel_buffer_layout(sps: &Sps) -> Result<PixelBufferLayout, ParseError> {
    validate_bit_depth_for_profile(sps.profile_idc, sps.bit_depth_luma, sps.bit_depth_chroma)?;

    let (chroma_w_shift, chroma_h_shift) = match sps.chroma_format_idc {
        0 => (None, None),      // monochrome: 크로마 평면 없음
        1 => (Some(1), Some(1)), // 4:2:0
        2 => (Some(1), Some(0)), // 4:2:2
        3 => (Some(0), Some(0)), // 4:4:4
        other => return Err(ParseError::InvalidChromaFormat { value: other }),
    };

    let bytes_per_sample = if sps.bit_depth_luma > 8 || sps.bit_depth_chroma > 8 { 2 } else { 1 };
    PixelBufferLayout::new(sps.pic_width, sps.pic_height, chroma_w_shift, chroma_h_shift, bytes_per_sample)
}
```
- `chroma_format_idc` 값을 명시적으로 매칭해 크로마 서브샘플링 shift를 유도하고, 이 값을 검증 없이 하드코딩한 상수(`3/2`)로 대체하지 않는다.
- 프로파일별 bit depth 제약을 별도 검증 함수로 분리해(PARSE-032와 유사한 접근) 위반 시 경고를 남긴다.
- 여러 코덱이 공유하는 픽셀 버퍼 레이아웃 계산 유틸리티가 있다면, 그 유틸리티의 테스트 스위트에 4:2:0/4:2:2/4:4:4/monochrome × 8/10/12bit 조합을 모두 포함시켜 회귀를 방지한다.

**탐지 방법**:
- Runtime: chroma_format_idc와 bit_depth의 모든 유효 조합(및 몇 가지 프로파일 위반 조합)에 대해 버퍼 크기 계산 결과를 검증하는 조합 테스트(parametrized test).
- Semantic: 픽셀 버퍼 크기 계산 코드에서 `3 / 2`, `4:2:0` 같은 하드코딩된 서브샘플링 가정이 있는지 grep해 전수 조사.

**예외**:
- 애플리케이션이 명시적으로 "4:2:0 8bit만 지원"을 스코프로 선언하고, 다른 조합의 파일은 파싱 단계에서 조기에 명확히 거부(에러로)하는 정책이라면, 하드코딩 자체보다는 "그 가정이 깨지는 입력을 fatal로 처리하는 명시적 가드"가 있는지가 중요하다.

**Bitvue 판정**: Suspected (부분 확인, 2026-08-01) — SPS 파싱 단계에서 `bit_depth_luma`/`bit_depth_chroma`와 `chroma_format_idc`를 프로파일별로 교차검증하는 `validate_bit_depth_for_profile`류 함수는 코덱 크레이트에서 찾지 못함(부재). 다만 디코드 후단인 `crates/bitvue-decode/src/decoder.rs`의 `ChromaFormat::from_frame_data()`(라인 88-139)는 실제 U/V 플레인 크기를 `checked_mul`로 계산한 기댓값과 비교해 4:2:0/4:2:2/4:4:4를 동적으로 판별(하드코딩된 `* 3 / 2` 가정이 아님)해 이 문서가 우려하는 버퍼 오버런의 핵심 위험은 실질적으로 완화되어 있음. 단, 판별 실패 시 "assuming 4:2:0"으로 조용히 폴백(라인 130-138)하는 지점은 이 문서가 지적하는 위험과 유사한 잔여 리스크.

---

### PARSE-040: 매직 바이트 몇 개만으로 코덱/컨테이너를 감지해 잘못된 파서로 디스패치
**분류**: PARSE · **심각도**: High · **탐지**: Structural/Runtime

**나쁜 예**:
```rust
fn detect_and_parse(data: &[u8]) -> Result<ParsedStream, ParseError> {
    if data.len() >= 4 && &data[0..4] == b"DKIF" {
        return ivf::parse(data); // IVF 매직 확인 후 무조건 IVF 파서로 디스패치
    }
    if data.len() >= 3 && data[0] == 0 && data[1] == 0 && data[2] == 1 {
        return avc::parse(data); // 첫 3바이트가 start code면 AVC로 간주
    }
    // 이 휴리스틱은 HEVC Annex B(같은 00 00 01 start code)도 AVC로 오인식한다
    Err(ParseError::UnknownFormat)
}
```

**문제**:
- `00 00 01` start code는 AVC/HEVC/VVC가 모두 공유하는 패턴이므로, 처음 3바이트만으로 "AVC다"라고 단정하면 HEVC/VVC 파일이 잘못된 코덱 파서로 넘어가 완전히 엉뚱한 방식으로 해석된다 — 이는 malformed 입력이 아니라 정상 파일조차 잘못 처리하는 근본적 설계 결함이다.
- 여러 코덱을 지원하는 이 프로젝트 구조상(각 코덱이 독립 크레이트), "이 파일이 어느 크레이트로 가야 하는가"를 결정하는 디스패치 로직이 부정확하면, 이후의 모든 안전성 노력(각 크레이트 내부의 검증)이 애초에 잘못된 파서에 진입한 순간부터 무의미해진다 — 잘못된 파서가 malformed 입력을 받는 것과 동일한 상황이 되어 이 카탈로그의 다른 모든 항목(PARSE-001~039)이 동시에 발동할 수 있는 조건을 스스로 만든다.
- 실제로 AVC와 HEVC를 구분하려면 start code 다음의 NAL 헤더 구조 자체가 다르다는 점(AVC는 1바이트 NAL 헤더, HEVC는 2바이트)과 그 안의 `nal_unit_type` 값 범위를 함께 봐야 하는데, 이런 실질적 구분 로직 없이 "매직 바이트 존재 여부"만으로 조기 결정하는 것이 문제의 핵심이다.

**발생 조건**:
- 컨테이너 없이 raw 코덱 스트림만 제공되는 경우(Annex B 등), 컨테이너 메타데이터(코덱 FourCC)에 의존할 수 없어 휴리스틱 감지가 필요한 상황.
- 파일 확장자가 실제 내용과 다르거나 없는 경우(사용자가 확장자를 임의로 바꾼 파일).

**권장**:
```rust
fn detect_and_parse(data: &[u8]) -> Result<ParsedStream, ParseError> {
    if data.len() >= 4 && &data[0..4] == b"DKIF" {
        return ivf::parse(data);
    }
    if let Some(start_code_pos) = find_first_start_code(data) {
        // 여러 코덱 후보에 대해 "이 후보로 해석했을 때 처음 몇 개 NAL이 구조적으로 유효한가"를
        // 실제로 시도해보고, 성공한(그리고 가장 그럴듯한) 후보를 채택한다
        for candidate in [CodecCandidate::Avc, CodecCandidate::Hevc, CodecCandidate::Vvc] {
            if let Ok(confidence) = probe_nal_structure(data, start_code_pos, candidate) {
                if confidence.is_high_enough() {
                    return candidate.parse(data);
                }
            }
        }
    }
    Err(ParseError::AmbiguousOrUnknownFormat {
        checked_candidates: vec!["ivf", "avc", "hevc", "vvc"],
    })
}
```
- 단순 매직 바이트 매칭이 아니라, 각 코덱 후보에 대해 "처음 N개 NAL을 이 코덱의 규칙으로 파싱했을 때 구조적으로 말이 되는가"(NAL 헤더 크기, `nal_unit_type` 값이 그 코덱에서 정의된 범위 안인가, 첫 NAL이 보통 SPS/VPS 타입인가)를 실제로 시도해보는 확률적/검증적 감지(probing)를 사용한다.
- 모든 후보가 실패하거나 여러 후보가 동시에 "그럴듯"하면, 함부로 하나를 선택하지 말고 사용자에게 "형식을 확정할 수 없습니다"를 명시적으로 알리고 수동 선택 옵션을 제공한다 — 잘못된 자동 감지보다 명확한 실패가 훨씬 안전하다.
- 컨테이너에 코덱 정보(FourCC, `CodecID` 등)가 있는 경우 그것을 최우선 신뢰 소스로 삼고, raw 스트림 휴리스틱 감지는 그 정보가 없을 때의 최후 수단으로만 사용한다.

**탐지 방법**:
- Structural: 포맷/코덱 감지 진입점 함수를 찾아, 감지 로직이 "존재 여부 확인"에 그치는지 "구조적 검증"까지 하는지 리뷰.
- Runtime: AVC/HEVC/VVC 각각의 실제 Annex B 파일을 서로의 파서로 강제 디스패치했을 때 무엇이 일어나는지(조용히 잘못된 결과를 내는지, 아니면 명확히 실패하는지) 회귀 테스트로 확인.
- Manual: 확장자를 의도적으로 바꾼 테스트 파일(`.hevc`를 `.264`로 리네임 등)로 감지 로직의 강건성을 수동 검증.

**예외**:
- 컨테이너가 코덱을 명시적이고 신뢰 가능하게 선언하는 경우(MP4의 `stsd` 내 codec FourCC 등)는 이런 휴리스틱 자체가 필요 없다 — 이 항목은 코덱 정보가 없는 raw/컨테이너리스 스트림에만 해당한다.

**Bitvue 판정**: Suspected (부분 확인, 2026-08-01) — 이 문서의 나쁜 예가 지적하는 구체적 버그("00 00 01" 3바이트만으로 AVC로 단정)는 `src-tauri/src/commands/analysis/mod.rs`의 `detect_codec_from_data()`에서 발견되지 않음(Annex B 스타트코드 자체를 이 함수가 별도 처리하지 않고 "unknown" 처리). 대신 `detect_codec_from_path()`(라인 293-310)가 파일 확장자를 우선 신뢰(예: `.264`→avc, `.265`→hevc)하고 실패 시에만 `detect_codec_from_content()`로 폴백하는 구조라, 확장자가 실제 내용과 다르면(사용자가 리네임한 파일 등) 오디스패치 위험이 남아있음 — 이는 이 항목이 우려하는 근본 문제(신뢰할 수 없는 약한 신호로 파서를 결정)의 다른 변형. 또한 `detect_codec_from_data()`가 MP4 `ftyp` 박스에서 코덱 FourCC(`vvc1`/`hvc1`/`avc1`/`av01`/`av03`)를 못 찾으면 `return "vvc".to_string()`으로 근거 없이 VVC를 기본값으로 반환(라인 361-362 부근)하는 것도 위험한 blind fallback으로 보임.

---

