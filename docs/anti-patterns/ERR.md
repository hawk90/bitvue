# Anti-Pattern Catalog — ERR: Error·panic·복구

이 문서는 Bitvue 안티패턴 카탈로그의 한 분류(ERR)이며, 전체 목록은 `docs/anti-patterns/INDEX.md`(별도 작성 중)를 참고한다. 1단계(일반 참조 카탈로그) 산출물로, 아직 Bitvue 저장소를 감사하지 않은 상태에서 작성되었다.

## panic 전략은 바이너리마다 다르다

Bitvue는 최소 세 종류의 실행 산출물을 가질 수 있다: (1) 코덱 파싱/분석 로직을 담은 **라이브러리 크레이트**, (2) 그 라이브러리를 감싸는 **Tauri 데스크톱 앱**, (3) 배치 분석·CI 검증용 **CLI 툴**. `panic = "abort"`를 `Cargo.toml`의 `[profile.release]`에 전역으로 걸면 세 산출물 모두에 동일하게 적용되는데, 이는 성격이 전혀 다른 세 워크로드를 하나의 정책으로 묶는 것이다.

- **라이브러리 크레이트**: `panic = "abort"`를 지정할 수 없다(지정해도 무시됨 — 이 설정은 바이너리/cdylib 최종 링크 단계에서만 유효). 라이브러리는 `catch_unwind` 경계를 제공해 호출자가 panic을 흡수할 수 있게 설계해야 한다.
- **CLI 원샷 툴**: 입력 파일 하나를 분석하고 종료하는 프로세스라면 `abort`가 합리적이다. panic이 나면 프로세스가 즉시 죽고, 종료 코드로 실패를 알리고, 사용자는 재실행하면 된다. unwind 비용(바이너리 크기, 약간의 런타임 오버헤드)을 지불할 이유가 없다.
- **Tauri 데스크톱 앱**: `abort`는 부적절한 경우가 많다. 사용자가 여러 파일을 열어 비교하거나, 하나의 세션에서 프로젝트/북마크/주석을 누적해온 상태에서 임의의(신뢰할 수 없는) 손상 파일 하나를 여는 순간 프로세스 전체가 즉사하면 세션의 모든 미저장 상태가 사라진다. 데스크톱 앱은 `panic = "unwind"`(기본값) + 워커 스레드/`catch_unwind` 경계 + 프로세스 자체의 워치독으로 "이 파일 하나의 파싱 실패"와 "앱 전체 종료"를 분리해야 한다.

이 트레이드오프는 아래 ERR-016에서 별도 항목으로 다시 다룬다. 나머지 항목들은 이 원칙을 전제로, 개별 에러 처리·복구 패턴을 다룬다.

---

### ERR-001: parser 내부 unwrap
**분류**: ERR · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
fn parse_sps(reader: &mut BitReader) -> Sps {
    let profile_idc = reader.read_bits(8).unwrap();
    let level_idc = reader.read_bits(8).unwrap();
    let sps_id = reader.read_ue().unwrap();
    let width = reader.read_ue().unwrap() * 16;
    let height = reader.read_ue().unwrap() * 16;
    Sps { profile_idc, level_idc, sps_id, width, height }
}
```

**문제**:
- 비트스트림 파서는 정의상 신뢰할 수 없는 바이트열을 소비한다. `reader.read_bits`가 스트림 끝을 넘거나 Exp-Golomb 코드가 잘못되어 `None`/`Err`를 반환하는 상황이 "예외"가 아니라 "일상"이다.
- `unwrap()`은 이 일상적인 실패를 즉시 panic으로 승격시킨다. 파일 하나가 1바이트만 잘려도 앱이 죽는다.
- 퍼징이나 실제 사용자가 제공한 손상 파일에서 가장 흔하게 재현되는 크래시 패턴이며, 스택 트레이스만 봐서는 "이 SPS가 왜 잘못됐는지"를 알 수 없다.

**발생 조건**:
- 스트림이 예상보다 짧게 잘린 truncated 파일
- 비트 단위 필드 값이 스펙 범위를 벗어나 이후 read_ue 등이 비정상적으로 큰 길이를 읽으려 할 때
- 컨테이너 파싱 단계에서 payload 크기 계산이 틀려 실제 데이터보다 큰 범위를 파서에 넘길 때

**권장**:
```rust
fn parse_sps(reader: &mut BitReader) -> Result<Sps, ParseError> {
    let profile_idc = reader.read_bits(8)
        .map_err(|e| ParseError::truncated("sps.profile_idc", reader.bit_pos(), e))?;
    let level_idc = reader.read_bits(8)
        .map_err(|e| ParseError::truncated("sps.level_idc", reader.bit_pos(), e))?;
    let sps_id = reader.read_ue()
        .map_err(|e| ParseError::truncated("sps.sps_id", reader.bit_pos(), e))?;
    let width = reader.read_ue()
        .map_err(|e| ParseError::truncated("sps.pic_width", reader.bit_pos(), e))?
        .checked_mul(16)
        .ok_or_else(|| ParseError::field_overflow("sps.pic_width", reader.bit_pos()))?;
    let height = reader.read_ue()
        .map_err(|e| ParseError::truncated("sps.pic_height", reader.bit_pos(), e))?
        .checked_mul(16)
        .ok_or_else(|| ParseError::field_overflow("sps.pic_height", reader.bit_pos()))?;
    Ok(Sps { profile_idc, level_idc, sps_id, width, height })
}
```
- 모든 파서 진입점은 `Result`를 반환하고, `?`로 조기 반환한다.
- 산술 연산은 `checked_*`로 감싸 오버플로도 파싱 오류로 취급한다.
- `unwrap`/`expect`는 "이 값은 코드 상 논리적으로 절대 실패할 수 없음이 타입 시스템/사전 검증으로 보장된 경우"에만 예외적으로 허용하고, 그 근거를 주석으로 남긴다.

**탐지 방법**:
- `clippy::unwrap_used`, `clippy::expect_used` lint를 parser 크레이트에 `#![deny(...)]`로 강제
- CI에 `grep -rn "\.unwrap()\|\.expect(" src/parser/` 게이트 추가(화이트리스트 파일 제외)
- 퍼징(cargo-fuzz/AFL)으로 truncated/랜덤 바이트 입력을 넣어 panic 발생 여부 확인

**예외**:
- `#[cfg(test)]` 테스트 코드 내부의 unwrap
- 상수 리터럴이나 컴파일 타임에 유효성이 보장된 값(`Regex::new(r"^\d+$").unwrap()` 같은 정적 패턴 컴파일)
- 프로세스 시작 시 1회만 실행되는 설정 로딩 코드(단, 사용자 입력이 아닌 경우에 한함)

**Bitvue 판정**: N/A — **재감사(2026-08-18, Electron/sidecar 아키텍처 기준)**: 이 문서의 기존 판정은 2026-08-08 Tauri→Electron 마이그레이션(`src-tauri` 완전 삭제, `bitvue-core`→`bitvue-engine` 리네임, commit `e7194cc`) 이전 스냅샷을 근거로 작성돼 있었다 — `src-tauri/src/commands/`, `crates/bitvue-core`는 현재 저장소에 존재하지 않는다(재감사 계기: CLAUDE.md 지시대로 존재하지 않는 경로를 인용하고 있음을 grep으로 확인). 현재 코드 기준 재검증: `#[cfg(test)]`/doc-comment 블록을 걸러내는 스크립트로 표본 크레이트(`bitvue-av1-codec`, `bitvue-avc`, `bitvue-hevc`, `bitvue-vp9`, `bitvue-vvc`, `bitvue-formats`)를 재검사한 결과 프로덕션 코드의 unwrap/expect는 극소수였다 — 예외는 `crates/bitvue-hevc/src/frames.rs:181-190`(`HevcFrameBuilder::build()`가 필드별 `.expect("X is required")` 사용)인데, 바로 위 doc comment(174-176행)에 `# Panics`로 문서화된 빌더 계약 위반(호출자가 setter를 안 부름) 시 패닉이지 원시 바이트스트림 읽기 실패가 아니다. 표본 크레이트 전체에서 `BitReader`/파싱 결과에 직접 `.unwrap()`을 건 사례는 발견되지 않았다. 다만 `bitvue-av1-codec` 한 크레이트에만 287건(대부분 doc-comment/`#[cfg(test)]`)이 있어 전수 검사는 아니다.

---

### ERR-002: malformed input을 unreachable로 처리
**분류**: ERR · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
fn nal_unit_type_name(nal_type: u8) -> &'static str {
    match nal_type {
        0..=31 => "VCL",
        32 => "VPS",
        33 => "SPS",
        34 => "PPS",
        35..=39 => "non-VCL",
        // 스펙상 nal_unit_type은 6비트이므로 0..=63만 가능하다고 "확신"
        _ => unreachable!("invalid nal_unit_type: {}", nal_type),
    }
}
```

**문제**:
- "스펙상 이 값의 범위는 0..63이다"는 인코더가 스펙을 준수했다는 가정 위에 서 있다. Bitvue는 임의의(잠재적으로 스펙을 어기거나 의도적으로 조작된) 파일을 열어야 하므로 이 가정 자체가 안전하지 않다.
- 실제로는 비트 파싱 자체가 어긋나 엉뚱한 위치에서 이 필드를 읽었을 때(즉 상위 파싱이 이미 틀어졌을 때) 이 `unreachable!`이 최초의 가시적 증상이 되는 경우가 많다 — 근본 원인은 다른 곳인데 여기서 crash가 난다.
- `unreachable!`은 `unwrap`보다 더 나쁜 시그널을 준다: "이 코드는 절대 여기 도달하지 않는다"는 개발자의 주장이 실제로는 신뢰할 수 없는 외부 입력에 의해 깨진 것이므로, 향후 유지보수자가 원인을 오판하기 쉽다.

**발생 조건**:
- 손상되거나 fuzz로 생성된 파일에서 예약(reserved)/미래 확장 필드 값이 나올 때
- 컨테이너 레벨 오프셋 계산이 틀려 실제로는 payload 중간을 헤더로 잘못 해석했을 때
- 코덱 버전/프로파일 확장으로 스펙 범위가 넓어졌는데 파서가 구버전 가정을 그대로 유지할 때

**권장**:
```rust
fn nal_unit_type_name(nal_type: u8) -> &'static str {
    match nal_type {
        0..=31 => "VCL",
        32 => "VPS",
        33 => "SPS",
        34 => "PPS",
        35..=39 => "non-VCL",
        40..=63 => "reserved/unspecified", // 스펙 예약 영역: 알려지지 않았을 뿐 유효한 입력
    }
}
```
- match는 "알 수 없는 값"을 위한 분기를 항상 데이터로 남긴다 — panic이 아니라 `"reserved"`/`Unknown(u8)` 같은 값으로 표현한다.
- 상위 파서가 이 필드를 신뢰하기 전에 별도로 range validation을 수행하고, 실패 시 `ParseError`로 보고한다.
- `unreachable!`/`unimplemented!`는 "입력에 의존하지 않는, 컴파일 타임에 소진성(exhaustiveness)이 증명된 경로"에서만 사용한다(예: enum match에서 이미 앞선 필터로 배제된 variant).

**탐지 방법**:
- `grep -rn "unreachable!\|unimplemented!\|todo!" src/parser/` 후 각 사용처가 입력 의존적인지 수동 검토
- 코드 리뷰 체크리스트에 "이 unreachable은 컴파일러가 증명 가능한가, 아니면 우리가 스펙을 신뢰한 것인가?" 질문 포함
- 퍼징으로 reserved 필드 값을 넣은 샘플을 다수 생성해 회귀 테스트에 추가

**예외**:
- 이미 `match`가 타입 레벨에서 exhaustive함이 보장된 경우(예: 이전 단계에서 `enum`으로 변환을 마친 뒤의 재-match)
- 함수 진입 시 `debug_assert!`로 사전조건을 문서화하는 용도(단, release 빌드에서 이 경로가 도달 불가능함을 별도로 증명한 경우에 한함)

**Bitvue 판정**: N/A — **재감사(2026-08-18)**: 현재 워크스페이스(`src-tauri` 삭제 후) 기준 `unreachable!`/`unimplemented!`/`todo!`는 non-test 코드에 30건이며, 그중 26건은 `crates/bitvue-av1-codec/src/symbol/cdf.rs`(예: 1480, 1503, 1522행 등)에 몰려 있다 — 전부 `match qcat.min(3) { 0 => .., 1 => .., 2 => .., 3 => .., _ => unreachable!() }` 형태로, `.min(3)` 클램프 직후의 match라 컴파일러가 증명 가능한 도달불가 분기다(스펙 CDF 테이블 조회용 quantizer-category 버킷). 나머지는 `crates/bitvue-avc/src/overlay_extraction.rs:963,975,986`(비트 조합 exhaustive match, 파일 경로/행 번호 이전 판정과 동일)과 `crates/bitvue-decode/src/decoder.rs:634`(직전 `match format`으로 이미 필터링된 뒤의 재-match)로 이전 판정과 동일한 근거다. 손상된 입력값이 직접 도달 가능한 `unreachable!`은 이번에도 발견되지 않았다.

---

### ERR-003: anyhow::Error로 전 계층 평탄화
**분류**: ERR · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
// container/mp4.rs
fn parse_box(data: &[u8]) -> anyhow::Result<Box> { /* ... */ }

// codec/hevc/sps.rs
fn parse_sps(data: &[u8]) -> anyhow::Result<Sps> { /* ... */ }

// commands/frame.rs (Tauri command)
#[tauri::command]
fn get_frame_info(path: String, index: u32) -> Result<FrameInfo, String> {
    let boxes = parse_box(&data)?;      // anyhow -> ?
    let sps = parse_sps(&sps_data)?;    // anyhow -> ?
    // 마지막에 .to_string()으로 뭉개서 프론트로 전달
    Ok(FrameInfo::from(boxes, sps))
        .map_err(|e: anyhow::Error| e.to_string())
}
```

**문제**:
- `anyhow::Error`는 "이 함수를 호출하는 쪽이 에러 종류를 구분해서 다르게 대응할 필요가 없다"는 것을 전제로 한 타입이다. 파서 라이브러리 내부, 컨테이너 파싱, 코덱 파싱, Tauri 커맨드 경계까지 전부 `anyhow`로 통일하면 UI 계층은 "truncated file"과 "지원하지 않는 profile"과 "내부 버그"를 문자열 패턴 매칭 없이는 구분할 수 없다.
- 호출자가 `match`로 복구 전략을 분기(예: "이 필드는 건너뛰고 계속 진행" vs "전체 중단")하고 싶어도 `anyhow::Error`는 구조화된 variant를 제공하지 않는다.
- 라이브러리 크레이트(파서 core)가 `anyhow`에 의존하면, 이 크레이트를 다른 프로젝트나 CLI에서 재사용할 때도 항상 `anyhow`를 함께 끌고 와야 한다.

**발생 조건**:
- 여러 명이 각자 다른 모듈을 작업하며 "일단 컴파일되게" `anyhow::Result`로 시그니처를 통일했을 때
- 프로토타입 단계에서 빠르게 `?` 전파만 되면 된다고 판단해 도입한 뒤 구조화 리팩터링을 미룬 경우
- 프론트엔드/UI에서 "실패했습니다"만 보여주면 된다고 오판했을 때(실제로는 사용자가 "이 파일이 왜 안 열리는지" 알고 싶어함)

**권장**:
```rust
// parser core 크레이트: 구조화된 에러 타입
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("truncated stream at offset {offset} while reading {field}")]
    Truncated { field: &'static str, offset: u64 },
    #[error("unsupported profile_idc={profile_idc} at offset {offset}")]
    UnsupportedProfile { profile_idc: u8, offset: u64 },
    #[error("field {field} out of range: {value} at offset {offset}")]
    FieldOutOfRange { field: &'static str, value: i64, offset: u64 },
}

// 애플리케이션(Tauri) 계층: 각 하위 에러를 자신의 도메인 에러로 승격
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("container parsing failed: {0}")]
    Container(#[from] ContainerError),
    #[error("codec parsing failed: {0}")]
    Codec(#[from] ParseError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[tauri::command]
fn get_frame_info(path: String, index: u32) -> Result<FrameInfo, AppErrorDto> {
    let boxes = parse_box(&data).map_err(AppError::from)?;
    let sps = parse_sps(&sps_data).map_err(AppError::from)?;
    Ok(FrameInfo::from(boxes, sps))
}
```
- 라이브러리/파서 계층은 `thiserror`로 구조화된 enum을 정의해 variant별 분기가 가능하게 한다.
- 애플리케이션 최상위(예: `main.rs`, 스크립트, 로그 출력)에서만 `anyhow`(또는 동등한 타입 소거 컨테이너)를 사용해 "출력을 위한" 평탄화를 한다.
- Tauri 커맨드 경계에서는 `serde::Serialize`가 가능한 DTO 에러 타입으로 변환해 프론트가 `kind` 필드로 분기할 수 있게 한다.

**탐지 방법**:
- `grep -rn "anyhow::Result\|anyhow::Error" src/parser/ src/codec/` — 파서/코덱 크레이트 내부에 anyhow 의존이 있으면 구조적 위반
- `cargo tree` / 크레이트별 `Cargo.toml`에서 parser 크레이트가 `anyhow`를 dependency로 갖는지 확인
- 코드 리뷰: 새 에러 variant 추가 시 "이 정보가 UI에서 구분될 필요가 있는가?"를 기준으로 anyhow vs thiserror 선택을 검토

**예외**:
- CLI 툴의 `main()` 최상위 레벨에서 다양한 하위 시스템 에러를 모아 종료 코드/메시지로만 출력할 때
- 프로토타입/스파이크 코드에서 임시로 사용하되, 정식 병합 전 구조화 타입으로 교체하기로 명시적으로 트래킹된 경우

**Bitvue 판정**: Confirmed — **재감사(2026-08-18)**: Electron 마이그레이션 이후 아키텍처가 크게 바뀌어 이전 판정(Tauri `Result<T,String>` 161곳, 죽은 `BitvueError`)은 더 이상 실재하는 경로를 가리키지 않는다(`src-tauri` 삭제됨). 현재는 오히려 이 항목의 "권장"에 가까운 인프라가 새로 생겼다: `crates/bitvue-protocol/src/lib.rs`가 `WireError`/`WireErrorCode`(Parse/Decode/NotFound/Cancelled/Internal 등, `BitvueError`의 variant를 의도적으로 미러링한 "안정된 wire 계약"이라고 문서화됨, 116-145행)를 정의하고, `bitvue-desktop/src/sidecarClient.ts:54-57`의 `SidecarRequestError`는 이 `error.code`를 타입 필드로 보존한다. 다만 실제 사용에서 여전히 이 항목이 경고하는 손실이 두 지점에서 재발한다: (1) `crates/bitvue-sidecar/src/main.rs`의 `get_coding_flow_analysis`(1095-1104행)/`get_deblocking_analysis`(1157-1165행)/`get_codec_extended_info`(1219-1227행) 세 핸들러 모두, 내부 `Result<Value, String>`의 실패 원인과 무관하게 무조건 `WireErrorCode::FrameNotFound`로 하드코딩해 응답한다 — 즉 파싱 실패든 코덱 미지원이든 진짜 "프레임 없음"이든 프론트는 전부 같은 코드로 받는다. (2) 그렇게 보존된 `error.code`조차 `frontend/services/electronBridgeService.ts`(현재 활성 커맨드 계층, 881줄)가 일반 `Error`로 던지기만 해서(예: 526행) UI까지 전달되지 않는다 — non-test 프런트엔드 코드 전체에서 `error.code`/`WireErrorCode`를 참조하는 곳이 전무하다(grep 무결과). 참고로 이전 판정이 인용한 `isNonRetriableError` 문자열매칭 패턴은 `frontend/services/tauriCommandService.ts`에 여전히 코드로 남아있지만 `@tauri-apps/api/core`를 import하는 죽은 파일이고(자기 테스트 파일 외 참조처 없음), 실제 앱은 `electronBridgeService.ts`를 쓴다.
**관련**: 배선 문제 관점은 `WIRING.md`의 WIRE-005 참고(현행화 필요할 수 있음 — 미확인).

---

### ERR-004: 오류에 file offset 없음
**분류**: ERR · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("truncated stream")]
    Truncated,
    #[error("invalid field value")]
    InvalidField,
}

fn parse_pps(reader: &mut BitReader) -> Result<Pps, ParseError> {
    let pps_id = reader.read_ue().map_err(|_| ParseError::Truncated)?;
    // ...
}
```

**문제**:
- 100MB 파일에서 "truncated stream"이라는 메시지만 보고는 어느 바이트에서 문제가 생겼는지 알 수 없다. 사용자가 hex viewer로 원인을 찾으려 해도 단서가 없다.
- 동일한 파일을 재분석해도 offset이 기록되지 않으면 매번 처음부터 이진 탐색하듯 원인을 찾아야 한다 — 버그 리포트를 받아도 재현 지점을 특정하기 어렵다.
- Bitvue처럼 "hex view와 파싱 결과를 연동해서 보여주는" 도구에서는 offset이 없는 에러는 도구의 핵심 가치(신택스 요소 ↔ 바이트 위치 매핑)를 스스로 포기하는 것과 같다.

**발생 조건**:
- 에러 타입을 빠르게 만들 때 메시지 문자열만 채우고 위치 정보 필드를 생략
- 여러 단계의 `?` 전파 과정에서 최초 발생 지점의 offset이 유실되고 마지막에 감싼 에러만 남을 때
- 비트 단위 파싱(바이트 offset이 아니라 bit offset이 필요한 경우)에서 "바이트 단위만 기록하면 되겠지"라고 단순화했을 때

**권장**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("truncated stream at byte {byte_offset}, bit {bit_offset} while reading {field}")]
    Truncated { field: &'static str, byte_offset: u64, bit_offset: u8 },
    #[error("invalid value for {field} at byte {byte_offset}: {value}")]
    InvalidField { field: &'static str, byte_offset: u64, value: i64 },
}

fn parse_pps(reader: &mut BitReader) -> Result<Pps, ParseError> {
    let (byte_offset, bit_offset) = reader.position();
    let pps_id = reader.read_ue().map_err(|_| ParseError::Truncated {
        field: "pps.pps_id",
        byte_offset,
        bit_offset,
    })?;
    // ...
    Ok(Pps { pps_id, /* ... */ })
}
```
- 모든 파서 에러 variant는 최소 `byte_offset`(가능하면 `bit_offset`까지)을 필드로 가진다.
- `BitReader`/`ByteReader`는 현재 위치를 O(1)로 조회할 수 있는 API(`position()`)를 제공해 에러 생성 지점에서 값싸게 얻을 수 있게 한다.
- 프론트엔드로 전달되는 에러 DTO에도 offset을 포함시켜, hex viewer가 해당 바이트로 자동 스크롤/하이라이트할 수 있게 한다.

**탐지 방법**:
- 에러 enum 정의를 grep해 offset류 필드(`offset`, `byte_offset`, `pos`)가 없는 variant를 찾아 리스트업
- 코드 리뷰 체크리스트: "이 에러가 UI에 노출됐을 때 사용자가 파일의 어느 지점을 봐야 하는지 알 수 있는가?"
- 통합 테스트에서 truncated 샘플을 넣고 반환된 에러 메시지에 offset 숫자가 포함되는지 assert

**예외**:
- 파일 전체에 대한 전역적 실패(예: "이 파일은 지원하지 않는 컨테이너 포맷입니다")처럼 특정 바이트 위치가 의미 없는 경우
- 사용자 입력 검증(예: CLI 인자 파싱)처럼 파일 offset 개념 자체가 없는 에러

**Bitvue 판정**: Confirmed — **재검증(2026-08-18)**: `crates/bitvue-hevc/src/error.rs`, `crates/bitvue-avc/src/error.rs`, `crates/bitvue-decode/src/decoder.rs`는 마이그레이션 대상이 아니었던(`src-tauri`가 아닌) 크레이트라 이전 판정 내용이 그대로 재확인된다. `HevcError`는 `UnexpectedEof(u64)`/`Parse{offset, message}`처럼 일부 variant에 offset을 담지만, `AvcError`는 `InvalidSps(String)`/`InvalidPps(String)`/`InvalidSliceHeader(String)`/`InvalidSei(String)` 등 대부분의 variant가 offset 없는 순수 문자열이다(`NotEnoughData{expected, got}`도 byte 개수일 뿐 위치는 아님, error.rs:7-30). `crates/bitvue-decode/src/decoder.rs:12-24`의 `DecodeError::Decode(String)`도 offset이 전혀 없다(파일/행 번호까지 이전 판정과 동일 — bitvue-decode는 이번 마이그레이션에서 이동/개명되지 않았다).

---

### ERR-005: codec/frame/field context 없음
**분류**: ERR · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("motion vector prediction failed")]
    MvPredictionFailed,
}

fn predict_mv(ctx: &MbContext) -> Result<MotionVector, DecodeError> {
    // 어떤 코덱, 어떤 프레임, 어떤 매크로블록/CTU인지 에러에 없음
    ctx.neighbor_mv().ok_or(DecodeError::MvPredictionFailed)
}
```

**문제**:
- 여러 코덱(AVC/HEVC/VP9/AV1)을 동시에 다루는 분석기에서 "motion vector prediction failed"만 보고는 AVC의 MB인지 HEVC의 CU인지조차 알 수 없다.
- 같은 파일 안에서도 프레임 수백~수천 개 중 몇 번째 프레임, 어떤 field(top/bottom, 인터레이스)에서 발생했는지 없으면 재현이 사실상 불가능하다.
- 버그 리포트나 로그에 이런 에러가 쌓이면 "같은 에러가 반복되는지, 다른 원인인지"조차 구분할 수 없어 통계적 분석(예: "이 코덱의 이 경로가 특히 취약하다")이 불가능해진다.

**발생 조건**:
- 코덱별 파서를 급하게 이식하면서 공통 에러 타입에 컨텍스트 필드를 채우지 않고 넘어갈 때
- 깊이 중첩된 호출 스택(슬라이스 → 매크로블록 → 서브블록)에서 최하위 함수가 상위 컨텍스트(frame_num, slice_type)를 모르는 채로 에러를 생성할 때
- 여러 코덱이 공유하는 제네릭 유틸리티 함수에서 "코덱 중립적"이어야 한다는 이유로 컨텍스트 전달을 생략할 때

**권장**:
```rust
#[derive(Debug, thiserror::Error)]
#[error("[{codec}] frame #{frame_index} ({field:?}) {kind}")]
pub struct DecodeError {
    codec: Codec,
    frame_index: u64,
    field: FieldType,
    kind: DecodeErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeErrorKind {
    #[error("motion vector prediction failed for block ({mb_x}, {mb_y})")]
    MvPredictionFailed { mb_x: u32, mb_y: u32 },
}

fn predict_mv(ctx: &MbContext) -> Result<MotionVector, DecodeError> {
    ctx.neighbor_mv().ok_or_else(|| DecodeError {
        codec: ctx.codec,
        frame_index: ctx.frame_index,
        field: ctx.field,
        kind: DecodeErrorKind::MvPredictionFailed { mb_x: ctx.mb_x, mb_y: ctx.mb_y },
    })
}
```
- 파이프라인 상위(프레임/슬라이스 레벨)에서 컨텍스트를 담은 `struct`로 하위 에러를 감싸고, 하위 함수는 자신의 지역 정보(블록 좌표 등)만 책임진다.
- `thiserror`의 `#[error(...)]` 포맷 문자열로 사람이 읽기 쉬운 요약을 자동 생성해 로그/UI에 그대로 노출 가능하게 한다.
- 컨텍스트 struct(코덱, 프레임 인덱스, 필드 타입)는 파이프라인 전체에서 공통 타입으로 재사용해 일관성을 유지한다.

**탐지 방법**:
- 에러 타입/variant 정의에서 `codec`, `frame_index`(또는 동등 필드)가 없는 것을 grep으로 찾기
- 실제 손상 파일로 에러 로그를 생성해보고, 로그만으로 "어떤 코덱, 몇 번째 프레임"을 답할 수 있는지 수동 점검
- 코드 리뷰: 새 DecodeError variant 추가 시 "코덱/프레임 컨텍스트가 상위에서 자동으로 채워지는가"를 확인

**예외**:
- 컨테이너 레벨(코덱 판별 이전) 에러처럼 아직 codec/frame 개념이 성립하지 않는 단계
- 순수 유틸리티 함수(비트 리더 등)에서 발생하는 에러는 프레임 컨텍스트를 모를 수 있으며, 이 경우 호출자가 감싸는 책임을 진다(단, 반드시 감싸야 함)

**Bitvue 판정**: Confirmed — **재검증(2026-08-18)**: `crates/bitvue-decode/src/decoder.rs:12-24`의 `DecodeError`(`Init(String)`/`Decode(String)`/`NoFrame`/`UnsupportedFormat`)는 이번 마이그레이션과 무관한 크레이트라 그대로 재확인된다 — codec 종류, frame_index, field 등 어떤 컨텍스트 필드도 없다. `crates/bitvue-decode/src/vvdec.rs`(dav1d/vvdec/ffmpeg 백엔드 모두)의 모든 디코드 실패가 이 평평한 문자열 하나로 귀결되어, `bitvue-sidecar`가 어떤 코덱·몇 번째 프레임에서 실패했는지 에러 타입만으로는 알 수 없다. (반면 `bitvue_core::CodecError`는 `bitvue-core`→`bitvue-engine` 리네임을 거쳐 `crates/bitvue-engine/src/codec_error.rs`로 이동했으며 `codec: Codec` 필드는 유지된다 — `UnexpectedEof{codec, position}` 등, 44행 이하 — 컨테이너/코덱 파서 레벨은 부분적으로 이 항목을 지킨다는 결론도 그대로 유효.)

---

### ERR-006: recoverable warning을 fatal 처리
**분류**: ERR · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn parse_slice_header(reader: &mut BitReader, sps: &Sps) -> Result<SliceHeader, ParseError> {
    let num_ref_idx = reader.read_ue()?;
    if num_ref_idx > sps.max_num_ref_frames {
        // 스펙 위반이지만 대부분의 디코더는 clamp해서 진행 가능
        return Err(ParseError::InvalidField {
            field: "num_ref_idx",
            value: num_ref_idx as i64,
            byte_offset: reader.byte_pos(),
        });
    }
    Ok(SliceHeader { num_ref_idx, /* ... */ })
}
```

**문제**:
- `num_ref_idx`가 SPS의 상한을 넘는 것은 흔한 인코더 버그/의도적 스트레스 테스트 패턴이며, 대부분의 실제 디코더는 이를 clamp하거나 무시하고 디코딩을 계속한다. 이를 즉시 `Err`로 전체 파싱을 중단시키면 나머지 정상적인 프레임 수천 개까지 분석 불가 상태가 된다.
- "일부 필드가 스펙을 살짝 벗어남"과 "스트림이 근본적으로 파싱 불가능함"을 구분하지 않으면, 분석기가 실제 인코더/방송 스트림에서 흔히 나오는 minor violation마다 전체 실패로 응답해 도구의 실용성이 떨어진다.
- 사용자 입장에서는 "이 파일은 열 수 없습니다"라는 메시지를 받지만 실제로는 99%의 프레임이 정상이었던 경우가 많다.

**발생 조건**:
- 스펙 준수 여부를 이진(binary)으로만 취급해 "위반 = 즉시 중단"으로 설계했을 때
- 방송/레거시 인코더가 만든, 스펙을 엄밀히 지키지 않지만 실무에서 널리 재생되는 스트림을 분석 대상으로 삼을 때
- 원 코덱 스펙과 실제 디코더 관행(reference decoder의 관대한 처리) 간 차이를 반영하지 않았을 때

**권장**:
```rust
fn parse_slice_header(
    reader: &mut BitReader,
    sps: &Sps,
    warnings: &mut Vec<ParseWarning>,
) -> Result<SliceHeader, ParseError> {
    let num_ref_idx_raw = reader.read_ue()?;
    let num_ref_idx = if num_ref_idx_raw > sps.max_num_ref_frames {
        warnings.push(ParseWarning::ClampedField {
            field: "num_ref_idx",
            raw_value: num_ref_idx_raw as i64,
            clamped_to: sps.max_num_ref_frames as i64,
            byte_offset: reader.byte_pos(),
        });
        sps.max_num_ref_frames
    } else {
        num_ref_idx_raw
    };
    Ok(SliceHeader { num_ref_idx, /* ... */ })
}
```
- 파서는 `Result<T, FatalError>`뿐 아니라 `Vec<ParseWarning>`(또는 `(T, Vec<Warning>)`)을 함께 반환해 non-fatal 이상 징후를 누적한다.
- "clamp 후 계속 진행"이 안전한 필드(범위를 벗어난 참조 인덱스 등)와 "더 이상 파싱 좌표계를 신뢰할 수 없는" 필드(길이/크기 필드처럼 이후 바이트 해석 자체가 틀어지는 것)를 사전에 분류해 정책을 문서화한다.
- UI에는 warning을 별도 심각도로 표시(노란색 등)하되 분석 자체는 끝까지 진행되게 한다.

**탐지 방법**:
- 파서 코드에서 스펙 범위 검증 후 즉시 `return Err`하는 위치를 모두 나열하고, 각각이 "이후 바이트 해석에 영향을 주는지" 여부로 fatal/recoverable을 재분류
- 실제 방송/레거시 샘플 파일 세트로 회귀 테스트를 돌려 "정상적으로 재생되는 파일인데 Bitvue만 실패"하는 케이스를 수집
- reference decoder(예: FFmpeg) 대비 "동일 파일에서 실패율" 비교

**예외**:
- 필드가 이후 파싱의 좌표계(길이, 오프셋, 카운트)를 결정하는 경우 clamp 자체가 위험할 수 있으므로 fatal 처리가 맞다
- 보안 강화가 목적인 엄격 모드(strict mode)가 명시적으로 요청된 경우(예: conformance 검사 도구로 사용할 때)

**Bitvue 판정**: Suspected — **재검증(2026-08-18)**: `Diagnostic`/`Severity` 인프라는 `bitvue-core`→`bitvue-engine` 리네임을 거쳐 `crates/bitvue-engine/src/event.rs`로 이동해 여전히 존재하고, `crates/bitvue-av1-codec/src/obu.rs`의 `parse_all_obus_resilient`(362행)도 그대로 남아있다. 다만 이 인프라가 AV1 이외 코덱(HEVC/AVC/VP9/VVC)에도 동일하게 적용되는지, 필드별 fatal/recoverable 분류가 스펙상 clamp 가능한 필드 기준을 실제로 따르는지는 여전히 코덱별 전수 검증을 하지 못했다(이전 판정과 결론 동일, 경로만 갱신).

---

### ERR-007: fatal corruption을 warning 처리
**분류**: ERR · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn parse_slice_data_offset(reader: &mut BitReader, warnings: &mut Vec<ParseWarning>) -> u64 {
    let len = reader.read_u32().unwrap_or(0);
    if len as usize > reader.remaining_bytes() {
        // "일단 경고만 남기고 최선을 다해 진행"
        warnings.push(ParseWarning::LengthExceedsBuffer { declared: len as u64 });
        return reader.remaining_bytes() as u64; // 임의로 clamp해서 계속 진행
    }
    len as u64
}
```

**문제**:
- 길이 필드(`len`)는 이후 모든 바이트 해석의 좌표계를 결정한다. 선언된 길이가 실제 버퍼보다 크다는 것은 "이 지점부터 우리가 읽는 바이트 위치 자체를 더 이상 신뢰할 수 없다"는 신호인데, 이를 조용히 clamp하고 warning만 남기면 이후 파싱 결과 전체가 잘못된 위치에서 잘못 해석된 값을 "정상"인 것처럼 보고하게 된다.
- 사용자는 warning 배지 하나만 보고 넘어가지만, 실제로는 프레임 크기·타임스탬프·MB 타입 오버레이 같은 후속 데이터가 전부 신뢰할 수 없는 상태다 — "일부만 틀렸다"가 아니라 "이후 전부가 의심스럽다".
- 이런 clamp-and-continue는 메모리 안전성 자체는 지키더라도(버퍼 밖을 읽지 않음), 분석 결과의 정합성을 조용히 깨뜨려 잘못된 결론(예: "이 프레임은 정상 디코딩됨")으로 이어질 수 있다.

**발생 조건**:
- 길이/오프셋/카운트 필드처럼 이후 파싱의 좌표계를 결정하는 필드가 범위를 벗어났을 때
- "일단 크래시만 안 나면 된다"는 방어적 코딩이 과도해져, 복구 불가능한 상태까지 무조건 clamp로 눙치려 할 때
- fatal/recoverable 분류 기준(ERR-006 참고)이 코드베이스에 문서화되어 있지 않아 각자 다른 기준으로 판단할 때

**권장**:
```rust
fn parse_slice_data_offset(reader: &mut BitReader) -> Result<u64, ParseError> {
    let len = reader.read_u32()?;
    if len as usize > reader.remaining_bytes() {
        return Err(ParseError::FatalCorruption {
            field: "slice_data.length",
            declared: len as u64,
            available: reader.remaining_bytes() as u64,
            byte_offset: reader.byte_pos(),
        });
    }
    Ok(len as u64)
}
```
- 좌표계를 결정하는 필드(길이/오프셋/카운트)의 범위 초과는 "복구 불가"로 분류해 즉시 `Err`를 반환한다.
- 다만 ERR-008과 결합해, 이 fatal 에러가 발생한 지점까지의 **이전** 파싱 결과(예: 앞선 슬라이스들)는 유효하므로 그것까지는 보존한다 — "이 NAL 단위/이 프레임부터 신뢰할 수 없다"는 경계를 명확히 구분해서 보고한다.
- fatal/recoverable 분류 기준을 코드베이스 문서(예: `docs/parsing-error-policy.md`)로 명문화해 팀 전체가 같은 기준을 쓰게 한다.

**탐지 방법**:
- 길이/오프셋/카운트 필드를 파싱하는 모든 지점에서 범위 검증 실패 시 `Err`가 아니라 `warnings.push` + clamp로 처리하는 코드를 grep으로 탐색
- 의도적으로 조작된 length 필드를 가진 fuzz 샘플로 "warning만 남고 계속 진행됨" + "이후 필드 값이 명백히 말이 안 됨"이 동시에 발생하는지 확인
- 코드 리뷰 체크리스트: "이 필드가 잘못되면 이후 몇 개의 필드/구조체가 영향을 받는가?"를 질문해 1개 이상이면 fatal 후보로 표시

**예외**:
- 필드가 순수하게 정보성(예: 디스플레이용 메타데이터 문자열 길이)이고 이후 바이트 오프셋 계산에 전혀 관여하지 않는 경우 warning으로 유지 가능
- 명시적으로 "best-effort 복구 모드"를 사용자가 선택했을 때(예: "손상된 파일이라도 최대한 보여주기" 옵션) — 단, 이 경우에도 결과에 "이 지점부터는 추정치"라는 표식이 UI에 남아야 한다

**Bitvue 판정**: Suspected — **재검증(2026-08-18)**: 현재 워크스페이스 기준으로도 길이/오프셋/카운트 필드를 조용히 clamp한 뒤 warning만 남기고 계속 진행하는 명시적 패턴은 타겟 grep으로 찾지 못했다. ERR-006과 마찬가지로 fatal/recoverable 분류가 코덱별로 일관되게 적용되는지 전수 검증은 못 했으므로 부재를 확정하기는 어렵다(결론 이전과 동일).

---

### ERR-008: 부분 분석 결과를 모두 폐기
**분류**: ERR · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn analyze_file(path: &Path) -> Result<AnalysisResult, AppError> {
    let mut frames = Vec::new();
    for nal in NalIterator::new(path)? {
        let frame = parse_frame(nal?)?; // 어느 한 프레임에서 에러 -> 함수 전체가 Err
        frames.push(frame);
    }
    Ok(AnalysisResult { frames })
}
```

**문제**:
- `?`를 반복문 안에서 그대로 사용하면 10,000프레임 중 9,999번째 프레임이 손상됐을 때 이미 성공적으로 파싱한 9,998개 프레임의 결과까지 통째로 버려진다.
- 사용자 입장에서는 "파일 전체를 분석할 수 없습니다"라는 결과만 받고, 실제로는 거의 끝까지 정상 분석됐다는 사실도, 어디서 문제가 생겼는지도 알 수 없다.
- Bitvue처럼 스트림 검증/디버깅이 목적인 도구에서 "끝부분 손상 때문에 앞부분도 못 봄"은 정확히 사용자가 가장 보고 싶어하는 정보(손상 직전까지의 정상 구조)를 숨기는 결과가 된다.

**발생 조건**:
- 스트리밍/캡처 중 잘린 파일(하드웨어 인코더 크래시, 네트워크 캡처 중단 등)
- 파일 뒷부분에만 국소적으로 비트 오류가 있는 경우(저장 매체 손상, 전송 오류)
- 반복문 내부의 개별 아이템 에러와 "더 이상 진행 불가능한" 전역 에러를 같은 `?`로 취급했을 때

**권장**:
```rust
fn analyze_file(path: &Path) -> AnalysisResult {
    let mut frames = Vec::new();
    let mut errors = Vec::new();

    let iter = match NalIterator::new(path) {
        Ok(it) => it,
        Err(e) => return AnalysisResult::failed(AppError::from(e)), // 컨테이너 자체가 안 열리면 fatal
    };

    for nal in iter {
        match nal.and_then(parse_frame) {
            Ok(frame) => frames.push(frame),
            Err(e) if e.is_recoverable() => {
                errors.push(e); // 이 프레임만 건너뛰고 계속
                continue;
            }
            Err(e) => {
                errors.push(e.clone());
                return AnalysisResult::partial(frames, errors); // fatal이면 여기까지 결과 보존
            }
        }
    }
    AnalysisResult::complete(frames, errors)
}
```
- 분석 결과 타입 자체가 `Complete { frames }` / `Partial { frames, errors, failed_at }` / `Failed { error }` 세 상태를 표현하게 설계한다.
- fatal 에러가 발생해도 그 지점까지 파싱된 프레임 리스트는 항상 반환값에 포함시킨다.
- UI는 partial 결과에 대해 "N/M 프레임 분석됨, 프레임 K에서 중단됨"과 같은 명확한 진행 상태를 표시한다.

**탐지 방법**:
- 반복문(프레임/슬라이스/NAL 단위) 내부에서 `?`를 사용해 함수 전체를 조기 종료시키는 패턴을 grep(`for .* {[\s\S]*?\?[\s\S]*?}`)으로 탐색 후 수동 검토
- truncated 파일(정상 파일을 임의 지점에서 자른 것)로 통합 테스트를 돌려, 잘린 지점 이전 프레임 수만큼 결과가 반환되는지 assert
- UI 통합 테스트: "일부만 분석됨" 상태가 실제로 화면에 프레임 목록과 함께 표시되는지 확인

**예외**:
- 컨테이너 헤더 자체를 파싱할 수 없어 프레임 경계를 하나도 찾을 수 없는 경우(반복 시작 전 실패) — 이때는 partial의 의미가 없으므로 전체 실패가 맞다
- 메모리/리소스 제약으로 partial 결과 보존 자체가 비용이 크다고 명시적으로 판단된 극단적 케이스(예: 수십 GB 파일의 전체 프레임 메타데이터를 메모리에 유지할 수 없는 경우) — 이 경우 스트리밍 방식으로 부분 결과를 순차 방출하는 대안을 검토해야 한다

**Bitvue 판정**: N/A — **재감사 결과 이전 Confirmed 판정을 뒤집음(2026-08-18)**: 이전 판정이 인용한 `src-tauri/src/commands/quality.rs`/`analysis/views.rs`는 Tauri→Electron 마이그레이션으로 완전히 삭제됐다. 현재 실사용 경로(`bitvue-sidecar`)를 재확인한 결과, 이 항목이 경고하는 "실패 시 전부 폐기" 패턴은 더 이상 존재하지 않는다: (1) `get_frame_analysis`가 의존하는 `crates/bitvue-av1-codec/src/overlay_extraction/parser.rs:259-271`은 fail-fast `parse_all_obus`를 먼저 시도하고, 실패 시 `ObuIterator::new(&obu_data).filter_map(|r| r.ok()).collect()`로 폴백해 성공적으로 파싱된 OBU만 모아 계속 진행한다(주석: "collect whatever OBUs parse successfully"). (2) `crates/bitvue-sidecar/src/{residual_analysis,coding_flow,deblocking,codec_extended_info}.rs`는 애초에 fail-fast `parse_all_obus().collect::<Result<_>>()`가 아니라 `ObuIterator`를 직접 순회하는 구조라 "OBU 하나 실패 시 전체 Err" 자체가 성립하지 않는다. 다만 이 폴백들이 개별 OBU 실패를 진단 정보 없이 그냥 건너뛴다는 점(ERR-020과 겹치는 더 경미한 별개 이슈)은 남아있다 — "부분 결과를 모두 폐기"라는 이 항목의 핵심 패턴만 놓고 보면 현재 코드에서 발견되지 않는다.

---

### ERR-009: worker 오류가 UI까지 전달되지 않음
**분류**: ERR · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
// Tauri 백그라운드 워커
fn spawn_analysis_worker(app: AppHandle, path: String) {
    std::thread::spawn(move || {
        match analyze_file(&path) {
            Ok(result) => {
                let _ = app.emit("analysis-complete", result);
            }
            Err(e) => {
                eprintln!("analysis failed: {e}"); // stderr에만 남고 끝
                // emit 호출이 없음 -> 프론트는 영원히 로딩 스피너만 봄
            }
        }
    });
}
```

```tsx
// 프론트엔드: analysis-complete만 구독
useEffect(() => {
  const unlisten = listen("analysis-complete", (e) => setResult(e.payload));
  return () => { unlisten.then(f => f()); };
}, []);
// analysis-error 이벤트를 구독하지 않으므로 실패 시 UI가 무한 로딩
```

**문제**:
- 워커 스레드에서 실패가 `eprintln!`으로만 기록되고 프론트로 통지되지 않으면, 사용자는 로딩 스피너가 영원히 도는 것만 본다 — 앱이 "멈춘 것"과 "실패한 것"을 구분할 수 없다.
- 개발자는 터미널에서 stderr를 보고 있으니 문제를 알아채지만, 실제 배포된 앱을 쓰는 사용자는 터미널을 보지 않는다.
- 이런 침묵 실패는 버그 리포트로도 이어지지 않는다(사용자가 "그냥 안 열려서 재시작했어요"라고만 인지하고 끝나는 경우가 많다) — 팀이 실패율을 파악할 방법 자체가 없어진다.

**발생 조건**:
- 백그라운드 스레드/워커에서 발생한 에러를 메인 스레드나 이벤트 루프로 명시적으로 전달하는 채널이 없을 때
- 성공 경로(이벤트 emit)만 구현하고 실패 경로(에러 이벤트)를 "나중에 추가"로 미뤄뒀다가 누락됐을 때
- `Result`를 반환하지 않는 `std::thread::spawn` 클로저 안에서 에러 처리가 로깅으로 끝나버리는 러스트 특유의 함정(스레드 panic도 마찬가지로 join하지 않으면 조용히 사라짐)

**권장**:
```rust
fn spawn_analysis_worker(app: AppHandle, path: String) {
    std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            analyze_file(&path)
        }));

        match outcome {
            Ok(Ok(result)) => {
                let _ = app.emit("analysis-complete", result);
            }
            Ok(Err(e)) => {
                log::error!("analysis failed for {path}: {e}");
                let _ = app.emit("analysis-error", AppErrorDto::from(&e));
            }
            Err(panic_payload) => {
                let msg = panic_message(&panic_payload);
                log::error!("analysis worker panicked for {path}: {msg}");
                let _ = app.emit("analysis-error", AppErrorDto::worker_panic(&msg));
            }
        }
    });
}
```
```tsx
useEffect(() => {
  const unlistenOk = listen("analysis-complete", (e) => setResult(e.payload));
  const unlistenErr = listen("analysis-error", (e) => setError(e.payload));
  return () => { unlistenOk.then(f => f()); unlistenErr.then(f => f()); };
}, []);
```
- 성공/실패 이벤트를 항상 쌍으로 설계하고, 둘 다 구독하는지 코드 리뷰에서 확인한다.
- 워커 내부는 `catch_unwind`로 panic까지 포착해 "에러"와 "패닉"을 모두 이벤트로 승격시킨다(패닉을 잡았다고 그 스레드의 내부 상태를 신뢰하고 재사용하면 안 됨 — ERR-012 참고).
- 타임아웃/워치독을 별도로 두어 이벤트조차 오지 않는 "완전 행(hang)" 상태도 UI에서 감지할 수 있게 한다.

**탐지 방법**:
- `std::thread::spawn`을 사용하는 모든 지점에서 클로저의 반환 경로(성공/실패/panic)가 전부 emit으로 이어지는지 코드 리뷰로 추적
- 의도적으로 손상된 파일을 열어 UI가 "로딩 중" 상태에서 멈추는지, 에러 메시지가 뜨는지 수동/E2E 테스트로 확인
- 프론트엔드 테스트에서 `analysis-error` 이벤트 구독이 존재하는지, mock emit에 반응하는지 검증

**예외**:
- 워커가 실패해도 UI에 아무 영향이 없는 순수 백그라운드 작업(예: 캐시 워밍, 텔레메트리 전송)은 로깅만으로 충분할 수 있다 — 단, 이 경우도 "사용자에게 영향 없음"이 실제로 참인지 검증되어야 한다

**Bitvue 판정**: Confirmed — **재검증(2026-08-18), 경로 갱신 + 새 아키텍처 함의 추가**: `bitvue-core`→`bitvue-engine` 리네임으로 파일은 `crates/bitvue-engine/src/worker.rs`로 이동했지만 문제의 코드는 그대로다. `AsyncJobManager::spawn`(371-373행)과 `complete_job`이 다음 큐 작업을 이어 실행하는 부분(433-435행) 모두 `std::thread::spawn(move || { f(); this.complete_job(&job_clone); })` 형태로 `catch_unwind`도 `JoinHandle` 보관도 없다. **다만 Electron 마이그레이션으로 이 버그의 실제 영향은 빌드 프로파일에 따라 달라진다**: 이 크레이트를 링크하는 `bitvue-sidecar`는 루트 `Cargo.toml:131`의 워크스페이스 전역 `panic = "abort"`를 상속하므로(ERR-016 참고), release 빌드에서는 `f()`가 panic하면 스레드가 조용히 죽는 게 아니라 **sidecar 프로세스 전체가 abort**한다 — `bitvue-desktop/src/sidecarClient.ts`가 이를 감지해 자동 재시작하지만(`sidecar.on("exit", ...)`, `restart_failed` 이벤트도 존재), 그 스트림에 물려있던 세션 상태는 전부 유실된다("nothing to replay them from"이 코드 주석에 명시). dev/debug 빌드(panic=unwind 기본값)에서는 원래 판정대로 해당 스레드만 조용히 죽어 `complete_job`이 안 불리고 in-flight 슬롯이 영구 누수된다.

---

### ERR-010: FFI 오류 코드를 문자열만으로 변환
**분류**: ERR · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
extern "C" {
    fn decoder_decode_frame(ctx: *mut DecoderCtx, out: *mut FrameBuf) -> i32;
}

fn decode_frame(ctx: &mut DecoderCtx, out: &mut FrameBuf) -> Result<(), String> {
    let ret = unsafe { decoder_decode_frame(ctx, out) };
    if ret != 0 {
        return Err(format!("decoder error code {ret}"));
    }
    Ok(())
}
```

**문제**:
- C 라이브러리의 정수 에러 코드를 그대로 문자열에 박아버리면, 호출자는 "이 코드가 재시도 가능한 일시적 오류인지, 스트림이 근본적으로 디코딩 불가능한 것인지"를 구분할 방법이 없다.
- FFI 라이브러리(예: dav1d, libvpx, libde265)는 보통 코드별 의미가 문서화된 enum/define을 제공하는데, 이를 매핑하지 않으면 매번 사람이 헤더 파일을 뒤져 코드 의미를 찾아야 한다.
- 문자열 비교로 상위 로직이 분기해야 하는 상황(`if err.contains("EAGAIN")`)이 생기면 취약하고 리팩터링에도 깨지기 쉬운 코드가 된다.

**발생 조건**:
- 서드파티 디코더 라이브러리를 FFI로 감쌀 때 에러 코드 매핑 테이블을 만들지 않고 "일단 동작하게" 정수만 넘길 때
- 여러 사람이 서로 다른 FFI 바인딩을 급하게 추가하면서 각자 임시방편으로 처리할 때
- 라이브러리 업데이트로 에러 코드 의미가 바뀌었는데 매핑이 갱신되지 않아 조용히 틀린 해석을 할 때

**권장**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderErrorCode {
    Eagain,          // 더 많은 입력 필요 — 재시도 가능
    InvalidBitstream, // 복구 불가능한 스트림 손상
    Unsupported,      // 이 프로파일/레벨 미지원
    OutOfMemory,
    Unknown(i32),
}

impl From<i32> for DecoderErrorCode {
    fn from(code: i32) -> Self {
        match code {
            -11 /* EAGAIN */ => Self::Eagain,
            -22 /* EINVAL */ => Self::InvalidBitstream,
            -95 /* ENOSYS */ => Self::Unsupported,
            -12 /* ENOMEM */ => Self::OutOfMemory,
            other => Self::Unknown(other),
        }
    }
}

fn decode_frame(ctx: &mut DecoderCtx, out: &mut FrameBuf) -> Result<(), DecodeError> {
    let ret = unsafe { decoder_decode_frame(ctx, out) };
    if ret != 0 {
        let code = DecoderErrorCode::from(ret);
        return Err(DecodeError::Ffi { code, raw: ret, library: "libdecoder" });
    }
    Ok(())
}
```
- FFI 크레이트마다 원본 라이브러리 헤더의 에러 코드 상수를 Rust enum으로 명시적으로 매핑하고, 미지의 코드는 `Unknown(i32)`로 원본 값을 보존한다.
- 매핑 테이블은 라이브러리 버전과 함께 버전 관리하고, 의존성 업그레이드 시 diff를 확인하는 체크리스트 항목으로 둔다.
- 재시도 가능(`Eagain`) / 즉시 포기(`InvalidBitstream`) / 기능 부재(`Unsupported`)를 타입으로 구분해 상위 로직이 `match`로 처리 전략을 명확히 분기하게 한다.

**탐지 방법**:
- `extern "C"` 블록과 그 호출부를 grep해 반환 정수를 `format!("{ret}")` 또는 `.to_string()`으로만 감싸는 지점 탐색
- FFI 라이브러리 헤더의 에러 코드 상수 목록과 Rust 매핑 enum의 variant 수를 대조해 누락된 코드 확인
- 통합 테스트에서 의도적으로 실패를 유발하는 입력(지원하지 않는 프로파일 등)을 넣어 반환된 에러가 올바른 variant로 분류되는지 확인

**예외**:
- 프로토타입 단계에서 FFI 바인딩을 처음 연결할 때 임시로 raw code만 넘기되, 정식 매핑 작업을 별도 이슈로 추적하는 경우
- 에러 코드가 단 하나(성공/실패 이진)뿐이고 향후에도 세분화될 가능성이 없다고 라이브러리 문서에 명시된 경우

**Bitvue 판정**: Suspected — **재검증(2026-08-18)**: `crates/bitvue-decode/src/vvdec.rs`는 이번 마이그레이션과 무관해 그대로 재확인된다(파일 성장으로 행 번호만 725-751로 소폭 이동). `VVDEC_OK`/`VVDEC_TRY_AGAIN`/`VVDEC_EOF`를 명시적으로 구분해 재시도 가능/스트림 종료를 구분하는 등 부분적으로 이 항목의 권장안을 따르지만, 그 외 모든 vvdec 에러 코드는 `_ => Err(DecodeError::Decode(Self::error_message(ret)))`(751행)로 단일 문자열에 뭉쳐진다(완전한 `DecoderErrorCode` enum 매핑은 없음). dav1d/ffmpeg-next 바인딩은 Rust 크레이트가 이미 에러를 감싸므로 이 항목이 직접 적용되는 raw FFI 코드 경로는 vvdec.rs가 여전히 유일하다.

---

### ERR-011: panic hook만 설치하고 실제 복구 없음
**분류**: ERR · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn main() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("panic occurred: {info}");
        // 로그만 남기고 끝 — 이후 프로세스는 기본 동작(스레드면 해당 스레드 종료,
        // 메인 스레드면 프로세스 종료)을 그대로 따름. UI에는 아무 통지도 없음.
    }));

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_frame_info, /* ... */])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**문제**:
- panic hook을 설치하는 것은 "panic이 발생했다는 사실을 기록"하는 것이지 "panic으로부터 복구"하는 것이 아니다. 이 둘을 혼동하면 "우리는 panic hook이 있으니 안전하다"는 잘못된 안도감을 갖게 된다.
- 워커 스레드에서 panic이 나면 hook이 로그를 남긴 뒤 해당 스레드는 여전히 종료된다. 그 스레드가 담당하던 작업(예: 진행 중인 파일 분석)의 호출자가 `JoinHandle::join()`으로 결과를 확인하지 않으면, UI는 이 스레드가 죽었다는 사실조차 모른 채 응답을 무한정 기다린다.
- 메인 스레드에서 panic이 나면 hook이 아무리 정교해도 `panic = "abort"` 설정 하에서는 프로세스가 즉시 종료된다 — 로그를 남기는 것과 세션을 살리는 것은 별개의 문제다.

**발생 조건**:
- "크래시 리포팅을 붙였으니 panic 대응은 끝났다"고 오판했을 때
- 워커 스레드의 `JoinHandle`을 버려두고(`let _ = thread::spawn(...)`) join하지 않아 스레드 panic이 아무 데도 전파되지 않을 때
- Sentry/crash reporter 같은 외부 서비스 연동을 "에러 처리"의 전부라고 여길 때

**권장**:
```rust
fn main() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("panic: {info}");
        telemetry::report_panic(info); // 진단 목적: 여기까지는 OK
    }));

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_frame_info, /* ... */])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 실제 복구는 panic이 발생할 수 있는 경계마다 catch_unwind로 구현
#[tauri::command]
fn get_frame_info(path: String, index: u32) -> Result<FrameInfo, AppErrorDto> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        analyze_frame(&path, index)
    }))
    .unwrap_or_else(|payload| {
        Err(AppError::WorkerPanic { message: panic_message(&payload) })
    })
    .map_err(AppErrorDto::from)
}
```
- panic hook은 "관측(로깅/텔레메트리)"용으로만 쓰고, "복구"는 위험한 호출(파서 진입점, FFI 호출, Tauri 커맨드 경계)마다 `catch_unwind`로 별도 구현한다.
- 워커 스레드는 반드시 `JoinHandle`을 보관하고 join 결과(`Result<T, Box<dyn Any + Send>>`)를 확인해 panic을 애플리케이션 에러로 승격시킨다(ERR-009와 연결).
- `catch_unwind`로 잡은 이후에는 그 경계 안에서 사용되던 공유 상태(락, 파서 인스턴스 등)를 재사용하지 않는다(ERR-012 참고) — panic은 불변식이 깨졌을 수 있다는 신호이기 때문이다.

**탐지 방법**:
- `set_hook`을 grep해 설치 여부를 확인한 뒤, hook 근처에 실제 `catch_unwind` 사용처가 있는지(같은 크레이트 내에) 대조
- `thread::spawn`의 반환값(`JoinHandle`)이 버려지는지(`let _ = ...` 또는 바인딩 없음) 정적 검사
- 의도적으로 파서 내부에서 panic을 유발하는 테스트(예: 배열 인덱스 초과)를 넣고, 앱이 해당 요청만 실패로 응답하며 세션이 유지되는지 통합 테스트로 확인

**예외**:
- CLI 원샷 툴처럼 panic 시 프로세스가 종료되어도 무방한 산출물에서는 hook을 로깅 전용으로만 두고 catch_unwind를 생략해도 된다(ERR-016 참고)
- 이미 `catch_unwind`로 감싸진 경계 내부의 하위 호출은 다시 감쌀 필요 없다(중첩 catch_unwind는 대개 불필요한 복잡도)

**Bitvue 판정**: N/A — **재검증(2026-08-18)**: `src-tauri`는 삭제됐지만 결론은 동일하다 — 현재 워크스페이스(`crates/bitvue-sidecar/src/main.rs` 포함, Electron `bitvue-desktop/electron/*.ts`도 함께 확인) 전체에서 `std::panic::set_hook` 호출은 어디에도 없다(grep 무결과). "hook은 설치했지만 실제 복구가 없다"는 정확한 패턴은 hook 자체가 부재하므로 성립하지 않는다. 실질적으로 더 심각한 문제(hook도 catch_unwind도 전혀 없음)는 여전히 ERR-009에서 확인된다 — `crates/bitvue-engine/src/worker.rs`의 스레드 panic이 어디에도 포착되지 않는다.

---

### ERR-012: 오류 발생 후 parser state 재사용
**분류**: ERR · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

fn parse_all_slices(reader: &mut BitReader, count: usize) -> Vec<SliceHeader> {
    let mut results = Vec::new();
    for _ in 0..count {
        match parse_slice_header(reader) {
            Ok(header) => results.push(header),
            Err(_) => {
                // 이 슬라이스는 실패했지만 reader는 그대로 두고 다음 반복으로 진행
                // -> reader.byte_pos/bit_pos가 실패 시점의 불확실한 위치에 멈춰 있음
                continue;
            }
        }
    }
    results
}
```

**문제**:
- 파싱 실패는 종종 "reader의 위치가 더 이상 신택스 구조와 정렬되어 있지 않다"는 것을 의미한다. 실패한 위치에서 그대로 다음 `parse_slice_header` 호출을 이어가면, 이후 호출은 완전히 엉뚱한 비트를 헤더 필드로 잘못 해석한다.
- 이런 "오정렬된 상태에서 계속 읽기"는 crash로 이어지지 않는 경우가 많아(단순히 잘못된 값을 반환) 더 위험하다 — 조용히 틀린 분석 결과를 만들어내고, 사용자는 이를 정상 결과로 오인할 수 있다.
- 캐시/재사용되는 파서 인스턴스(예: 세션 내내 유지되는 디코더 컨텍스트)에서 한 번의 실패가 이후 모든 프레임 파싱에 누적적으로 영향을 미칠 수 있다.

**발생 조건**:
- 반복문 안에서 실패를 "건너뛰기"로 처리하되 reader/커서 위치를 다음 유효한 동기화 지점(예: 다음 NAL 시작 코드, 다음 start code)으로 재정렬하지 않을 때
- 상태를 가진 파서 객체(스트리밍 파서, 증분 디코더)가 에러 이후에도 그대로 재사용되어 내부 불변식(예: "현재 활성 SPS/PPS는 유효하다")이 깨진 채로 남을 때
- `catch_unwind`로 panic만 잡고 그 안에서 사용되던 공유 mutable 상태(Mutex 내부 등)를 검증 없이 계속 사용할 때(ERR-011과 연결)

**권장**:
```rust
fn parse_all_slices(data: &[u8], boundaries: &[usize]) -> Vec<Result<SliceHeader, ParseError>> {
    boundaries.windows(2).map(|w| {
        // 매 슬라이스마다 독립된 reader를 그 슬라이스의 바이트 범위로 새로 생성
        // -> 한 슬라이스의 실패가 다음 슬라이스의 파싱 좌표계에 영향을 주지 않음
        let mut reader = BitReader::new(&data[w[0]..w[1]]);
        parse_slice_header(&mut reader)
    }).collect()
}

// 상태를 가진 파서라면: 실패 시 명시적으로 재동기화하거나 인스턴스를 폐기
impl StreamingParser {
    fn resync_to_next_start_code(&mut self) -> Result<(), ParseError> {
        let next = find_start_code(&self.buffer[self.pos..])
            .ok_or(ParseError::NoResyncPoint)?;
        self.pos += next;
        self.invalidate_cached_headers(); // SPS/PPS 캐시 등 불변식도 함께 리셋
        Ok(())
    }
}
```
- 가능하면 각 신택스 단위(슬라이스/NAL)를 독립된 바이트 범위로 미리 분리한 뒤 각각에 새 reader를 생성해, 한 단위의 실패가 다른 단위에 전혀 영향을 주지 않는 구조로 설계한다.
- 상태를 유지해야만 하는 스트리밍 파서라면, 실패 후 반드시 명시적 "재동기화(resync)" 절차(다음 start code/sync marker 탐색)를 거치고, 그 사이의 캐시된 파생 상태(활성 파라미터셋 등)도 함께 무효화한다.
- `catch_unwind`로 panic을 잡은 경계 안에서 사용된 공유 상태는 기본적으로 "오염된 것으로 간주"하고 폐기하거나 재초기화한다.

**탐지 방법**:
- 반복문에서 파싱 실패 시 `continue`만 하고 reader/커서 위치 재조정 코드가 없는 패턴을 코드 리뷰로 탐색
- fuzz 테스트: 스트림 중간에 임의로 바이트를 손상시킨 뒤, 손상 지점 이후 파싱 결과가 "명백히 말이 안 되는 값(음수 크기, 비현실적 해상도 등)"으로 나오는지 관찰 — 나온다면 재동기화 없이 계속 읽었다는 신호
- 상태를 가진 파서 구조체에 대해 "에러 반환 후 이 인스턴스를 그대로 재사용해도 안전한가?"를 타입/문서 레벨로 명시했는지 확인

**예외**:
- 프레임/NAL 단위가 이미 컨테이너 레벨에서 명확한 바이트 경계로 분리되어 있어 각 파싱이 처음부터 독립적인 경우(위 "권장" 예시처럼) — 이 경우 재동기화 문제 자체가 구조적으로 발생하지 않는다
- 실패 시 즉시 전체 파싱을 중단하는 정책(ERR-007의 fatal 케이스)에서는 "재사용"이 아예 일어나지 않으므로 해당 없음

**Bitvue 판정**: N/A — **재검증(2026-08-18)**: `crates/bitvue-av1-codec/src/obu.rs`의 `parse_all_obus_resilient`(경로/내용 변경 없음)는 실패 시 reader를 그 자리에 방치하지 않고 1바이트씩 전진하며 다음 유효 OBU 헤더를 재탐색하는 명시적 resync를 수행한다(10회 연속 실패 시 Fatal로 중단) — "권장" 예시의 취지와 일치, 이전 판정 그대로 유효. 추가로, ERR-008 재감사에서 확인했듯 현재 `bitvue-sidecar`의 여러 핸들러(`residual_analysis.rs`/`coding_flow.rs`/`deblocking.rs`/`codec_extended_info.rs`)가 쓰는 `ObuIterator` 직접 순회도 각 OBU를 독립적으로 처리해 실패한 OBU의 오염된 커서 상태가 다음 OBU로 전파되지 않는 구조다.

---

### ERR-013: 오류 종류별 retry 정책 없음
**분류**: ERR · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn decode_with_hw_accel(ctx: &mut HwDecoderCtx, packet: &Packet) -> Result<Frame, DecodeError> {
    hw_decode(ctx, packet) // 실패하면 그대로 상위에 전파 — 재시도도, 폴백도 없음
}
```

**문제**:
- 모든 실패를 동일하게 "즉시 포기"로 처리하면, 본질적으로 일시적인 실패(하드웨어 디코더 리소스 일시 부족, `EAGAIN`류)까지 영구적 실패처럼 사용자에게 보고하게 된다.
- 반대로 재시도 정책이 아예 없으면, 재시도해도 의미가 없는 실패(비트스트림 자체가 손상됨)에 대해서도 별도 처리 없이 상위로 던지기만 해 "왜 재시도하지 않았는지"에 대한 근거가 코드에 드러나지 않는다.
- 하드웨어 가속 디코더(GPU 자원 경합)나 외부 프로세스와의 IPC처럼 "다시 시도하면 성공할 수 있는" 클래스의 실패를 구분하지 않으면, 실제로는 복구 가능한 상황에서도 전체 분석이 실패로 끝난다.

**발생 조건**:
- 하드웨어 가속 디코딩 경로에서 GPU 컨텍스트/VRAM 경합으로 인한 일시적 실패
- 외부 프로세스(예: 별도 디코더 서브프로세스)와의 IPC 타임아웃
- 소프트웨어 폴백이 가능한 상황(HW 실패 시 SW 디코더로 전환)인데 이를 시도하지 않고 바로 실패 처리

**권장**:
```rust
#[derive(Debug, Clone, Copy)]
enum RetryPolicy {
    Never,                          // 예: InvalidBitstream — 재시도해도 항상 같은 결과
    Immediate { max_attempts: u8 }, // 예: Eagain — 즉시 재시도
    Fallback,                       // 예: 하드웨어 실패 -> 소프트웨어 디코더로 전환
}

fn retry_policy_for(err: &DecodeError) -> RetryPolicy {
    match err {
        DecodeError::Ffi { code: DecoderErrorCode::Eagain, .. } => {
            RetryPolicy::Immediate { max_attempts: 3 }
        }
        DecodeError::HwAccelUnavailable { .. } => RetryPolicy::Fallback,
        DecodeError::Ffi { code: DecoderErrorCode::InvalidBitstream, .. } => RetryPolicy::Never,
        _ => RetryPolicy::Never,
    }
}

fn decode_with_policy(ctx: &mut HwDecoderCtx, packet: &Packet) -> Result<Frame, DecodeError> {
    match hw_decode(ctx, packet) {
        Ok(frame) => Ok(frame),
        Err(e) => match retry_policy_for(&e) {
            RetryPolicy::Never => Err(e),
            RetryPolicy::Immediate { max_attempts } => retry_immediate(ctx, packet, max_attempts),
            RetryPolicy::Fallback => decode_with_sw_fallback(packet),
        },
    }
}
```
- 에러 variant마다 재시도 정책(재시도 안 함/즉시 재시도/폴백 경로)을 명시적으로 매핑하는 테이블을 코드로 남긴다.
- 재시도에는 항상 상한(`max_attempts`)을 둬 무한 루프를 방지하고, 재시도 사이 지수 백오프가 필요한 IO/네트워크성 실패와 즉시 재시도가 적절한 리소스 경합성 실패를 구분한다.
- 폴백 경로(HW → SW)를 탈 때는 그 사실을 로그/UI에 남겨 "정상 동작했지만 경로가 바뀌었다"는 것을 사용자가 알 수 있게 한다(성능 저하의 원인 파악에 중요).

**탐지 방법**:
- FFI/외부 리소스 호출부에서 실패 시 재시도 로직이 전혀 없는 지점과, 반대로 무조건 무한/과도하게 재시도하는 지점을 모두 코드 리뷰로 점검
- 하드웨어 가속 경로가 있는 코드베이스라면 "HW 실패 시 SW 폴백이 실제로 트리거되는지"를 하드웨어 디코더를 강제로 비활성화한 환경에서 통합 테스트로 확인
- 재시도 상한이 있는지, 백오프가 있는지 코드 정적 검사(재시도 루프에 `max_attempts`류 상수가 있는지)

**예외**:
- 순수 파싱(비트스트림 신택스 해석) 단계에는 대개 재시도 개념 자체가 성립하지 않는다(같은 바이트를 다시 읽어도 같은 결과) — 이 항목은 주로 IO, FFI, 하드웨어 리소스 경계에 적용된다
- 사용자가 명시적으로 "재시도 없이 즉시 실패 보고"를 선택하는 진단/디버그 모드에서는 정책을 의도적으로 `Never`로 고정할 수 있다

**Bitvue 판정**: Suspected — **재검증(2026-08-18)**: `crates/bitvue-decode/src/vvdec.rs`는 마이그레이션과 무관해 그대로 재확인된다. `MAX_TIMEOUT_RETRIES`(40행)/`DECODE_TIMEOUT`(46행)/`poisoned` 원자 플래그(268행) 기반의 타임아웃 재시도 로직이 존재해(get_frame 주변, 306-365행 및 621행 이후) 이 항목이 요구하는 것과 유사한 재시도 정책이 타임아웃 실패에 대해 부분적으로 구현되어 있다. 다만 에러 variant별 재시도 정책을 표(table)로 명문화한 구조는 아니고, HW→SW 폴백 같은 별도 경로가 실제로 존재/동작하는지는 여전히 확인하지 못했다.

---

### ERR-014: cancellation을 error로 기록
**분류**: ERR · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
async fn analyze_file(path: String, cancel: State<'_, CancelToken>) -> Result<AnalysisResult, String> {
    for chunk in chunks(&path) {
        if cancel.is_cancelled() {
            // 사용자가 취소 버튼을 눌렀을 뿐인데 "에러"로 기록됨
            log::error!("analysis failed: cancelled by user");
            return Err("analysis failed".to_string());
        }
        process_chunk(chunk)?;
    }
    Ok(collect_result())
}
```

**문제**:
- 사용자가 "취소" 버튼을 누른 것은 정상적인 사용자 상호작용이지 시스템의 실패가 아니다. 이를 에러 로그/에러 카운터에 함께 기록하면 실제 결함(파싱 버그, FFI 실패 등)의 발생률 통계가 취소 이벤트로 오염된다.
- UI가 에러 메시지("analysis failed")를 그대로 사용자에게 보여주면, 사용자는 자신이 취소했다는 것을 알면서도 "뭔가 잘못됐다"는 불필요한 불안을 느낀다.
- 크래시 리포팅/텔레메트리 시스템에 취소가 에러로 집계되면, 알림 임계치(예: "에러율 5% 초과 시 경보")가 실제로는 정상적인 대량 취소(예: 사용자가 큰 파일을 열다가 마음이 바뀐 경우)에 의해 오탐될 수 있다.

**발생 조건**:
- 취소 토큰 확인과 실제 에러 확인이 같은 `Result<T, E>` 타입/같은 에러 변환 경로를 공유할 때
- "일단 함수가 실패로 끝나는 경로는 다 Err로"라는 단순화된 사고로 취소를 별도 분류하지 않았을 때
- 취소 UI를 나중에 추가하면서 기존 에러 처리 인프라(로깅, 텔레메트리)에 취소 케이스를 새로 통합하지 않고 기존 에러 경로에 얹었을 때

**권장**:
```rust
enum AnalysisOutcome {
    Completed(AnalysisResult),
    Cancelled,                 // 사용자 의도 — 에러가 아님
    Failed(AppError),          // 실제 시스템/파싱 실패
}

#[tauri::command]
async fn analyze_file(path: String, cancel: State<'_, CancelToken>) -> Result<AnalysisOutcomeDto, String> {
    for chunk in chunks(&path) {
        if cancel.is_cancelled() {
            log::info!("analysis cancelled by user for {path}"); // info, error 아님
            return Ok(AnalysisOutcomeDto::Cancelled);
        }
        match process_chunk(chunk) {
            Ok(_) => {}
            Err(e) => {
                log::error!("analysis failed for {path}: {e}");
                return Ok(AnalysisOutcomeDto::Failed(AppErrorDto::from(e)));
            }
        }
    }
    Ok(AnalysisOutcomeDto::Completed(collect_result()))
}
```
- 취소는 `Err` 채널이 아니라 성공 채널의 별도 variant(`Cancelled`)로 표현해, 타입 시스템이 "이건 에러가 아니다"를 강제하게 한다.
- 로그 레벨도 구분한다 — 취소는 `info`/`debug`, 실제 실패는 `error`.
- 텔레메트리/에러율 집계 파이프라인에서 `Cancelled`를 실패 분모/분자 어디에도 포함시키지 않도록 명시적으로 필터링한다.

**탐지 방법**:
- 취소 토큰 확인 직후 `log::error!`나 에러 텔레메트리 호출이 있는지 grep으로 탐색
- 에러율 대시보드/알림 설정에서 "cancelled"라는 문자열을 포함한 에러가 실패 카운트에 섞여 있는지 실제 로그 샘플로 확인
- UI 테스트: 취소 버튼 클릭 시 사용자에게 노출되는 메시지가 "실패" 톤이 아니라 "취소됨" 톤인지 확인

**예외**:
- 취소 요청 자체의 처리 과정에서 별도 실패가 발생한 경우(예: 취소했는데 리소스 정리 중 IO 에러 발생)는 그 자체로는 정당한 에러이므로 별도 `Err`로 보고해도 된다 — 다만 "취소됨"과 "취소 처리 중 발생한 에러"를 구분해서 보고해야 한다

**Bitvue 판정**: N/A — **재검증(2026-08-18), 경로 갱신**: `bitvue-core`→`bitvue-engine` 리네임으로 파일 경로만 바뀌었다. 취소는 이미 전용 enum variant로 모델링되어 있다: `crates/bitvue-engine/src/worker.rs:128-132`의 `JobState::Cancelled`, `crates/bitvue-engine/src/index_session.rs`의 `IndexingState::Cancelled`가 `Error`와 별개 variant로 존재하며, `Result`의 `Err` 채널에 취소를 욱여넣는 코드는 이번에도 발견되지 않았다.

---

### ERR-015: 사용자 취소와 parser 실패 혼동
**분류**: ERR · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn analyze_with_cancel(path: &Path, cancel: &CancelToken) -> Result<AnalysisResult, ParseError> {
    let mut frames = Vec::new();
    for nal in NalIterator::new(path)? {
        if cancel.is_cancelled() {
            // 취소를 ParseError의 한 variant로 욱여넣음
            return Err(ParseError::Truncated {
                field: "cancelled",
                byte_offset: 0,
                bit_offset: 0,
            });
        }
        frames.push(parse_frame(nal?)?);
    }
    Ok(AnalysisResult { frames })
}
```

**문제**:
- 취소를 `ParseError::Truncated`처럼 원래 다른 의미(스트림이 잘렸음)를 가진 variant에 억지로 매핑하면, 이후 이 에러를 소비하는 모든 코드(로깅, UI, 재시도 정책 결정 — ERR-013)가 "진짜 truncated"와 "사용자가 취소함"을 구분할 수 없게 된다.
- 예를 들어 ERR-013의 재시도 정책 테이블에서 `Truncated`에 대해 "재시도 무의미(Never)"로 분류해뒀다면, 취소도 우연히 같은 경로를 타게 되어 정책 설계 의도와 실제 동작이 어긋난다.
- 버그 리포트 자동 수집 시스템이 "Truncated 에러가 급증했다"고 경보를 울렸는데, 실제로는 사용자들이 큰 파일 분석을 자주 취소한 것뿐인 상황이 발생할 수 있다(ERR-014와 동일한 통계 오염 문제가 여기서도 재발한다).

**발생 조건**:
- 기존 에러 enum에 취소를 위한 별도 variant를 추가하는 대신, 기존 variant 중 "의미상 비슷해 보이는" 것을 재사용할 때(코드 변경 최소화 유혹)
- 취소 가능한 장시간 작업을 나중에 추가하면서, 기존 함수 시그니처(`Result<T, ParseError>`)를 바꾸지 않고 그 안에 억지로 끼워 넣을 때
- 취소와 파싱 실패가 같은 반복문에서 함께 체크되어 코드 상으로도 개념이 뒤섞이기 쉬울 때

**권장**:
```rust
enum AnalysisControlFlow {
    Continue,
    Cancelled,
}

enum AnalysisResult2 {
    Completed(AnalysisResult),
    Cancelled,
    Failed(ParseError),
}

fn analyze_with_cancel(path: &Path, cancel: &CancelToken) -> AnalysisResult2 {
    let iter = match NalIterator::new(path) {
        Ok(it) => it,
        Err(e) => return AnalysisResult2::Failed(e),
    };

    let mut frames = Vec::new();
    for nal in iter {
        if cancel.is_cancelled() {
            return AnalysisResult2::Cancelled; // 별도 타입 — ParseError와 절대 섞이지 않음
        }
        match nal.and_then(parse_frame) {
            Ok(frame) => frames.push(frame),
            Err(e) => return AnalysisResult2::Failed(e),
        }
    }
    AnalysisResult2::Completed(AnalysisResult { frames })
}
```
- 취소는 `Result<T, E>`의 `E` 안에 절대 넣지 않는다 — 별도의 3-way(또는 그 이상) enum으로 "완료/취소/실패"를 표현한다.
- 함수 시그니처 변경이 부담스럽더라도, 취소를 기존 에러 타입에 끼워 넣는 것은 단기적 편의와 장기적 혼란을 맞바꾸는 것임을 리뷰에서 짚는다.
- 취소 체크와 파싱 에러 체크를 반복문 내에서 시각적으로도 분리(별도 `if`/`match` 분기)해 코드를 읽을 때부터 두 개념이 섞이지 않게 한다.

**탐지 방법**:
- 에러 enum의 각 variant 문서/주석에 "취소"라는 단어가 실패 의미와 함께 언급되어 있는지 검토(예: 위 나쁜 예의 `field: "cancelled"`처럼 필드 값으로 취소를 표현하는 편법)
- 취소 관련 필드/텍스트가 기존 에러 타입 안에 문자열/매직 값으로 숨어 있는지 grep(`"cancel"`, `"aborted"` 등 리터럴 검색)
- 재시도 정책(ERR-013)이나 에러 통계 집계 코드에서 취소 케이스가 실제로 별도 분기를 타는지 단위 테스트로 검증

**예외**:
- 없음 — 취소와 파싱 실패는 발생 원인, 사용자에게 보여줄 메시지, 통계적 취급이 모두 달라야 하므로 이 둘을 같은 타입으로 표현하는 것이 합리적인 상황은 사실상 없다. 다만 매우 단순한 CLI 툴에서 두 경우 모두 "종료 코드 1"로 귀결되는 최종 표현 단계에서는 합쳐도 무방하다(그 이전 단계까지 구분을 유지한 뒤 마지막에만 합치는 것이 핵심).

**Bitvue 판정**: N/A — **재검증(2026-08-18)**: ERR-014와 동일한 근거(경로는 `crates/bitvue-engine/src/worker.rs`, `index_session.rs`로 갱신). `JobState::Cancelled`/`IndexingState::Cancelled`가 `ParseError`/`DecodeError` variant로 재사용되는 사례를 찾지 못했으며, 취소는 처음부터 파싱/디코딩 에러 타입과 분리된 별도 상태 enum으로 설계되어 있다.

---

### ERR-016: panic = "abort"를 모든 바이너리에 획일 적용
**분류**: ERR · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```toml
# Cargo.toml (workspace root) — 데스크톱 앱, CLI, 라이브러리가 모두 이 profile을 공유
[profile.release]
panic = "abort"
lto = true
codegen-units = 1
```

**문제**:
- 워크스페이스 루트의 `[profile.release]`는 해당 profile로 빌드되는 모든 바이너리 타깃에 동일하게 적용된다. 바이너리별로 다른 정책이 필요하다는 것을 이 설정 하나로는 표현할 수 없다.
- 데스크톱 앱(Tauri) 바이너리에 `abort`가 적용되면, 워커 스레드에서 `catch_unwind`로 개별 파일 파싱 실패를 흡수하려 해도 **panic이 스레드 경계와 무관하게 프로세스를 즉시 종료**시킨다 — `catch_unwind`는 `unwind` 전략에서만 의미가 있고, `abort` 전략에서는 panic이 곧 프로세스 종료다. 즉 ERR-009, ERR-011에서 공들여 만든 "워커 실패를 UI로 안전하게 전달"하는 설계 전체가 무력화된다.
- 반대로 CLI 원샷 툴에 `unwind`를 쓰면 불필요하게 바이너리 크기가 커지고 약간의 런타임 비용을 지불하면서도, 얻는 이득(우아한 복구)은 어차피 프로세스가 곧 종료될 것이므로 거의 없다.

**발생 조건**:
- 워크스페이스 전체에 단일 `Cargo.toml` release profile을 두고, 바이너리 타깃별 특성을 고려하지 않았을 때
- "release 빌드는 최적화만 신경 쓰면 된다"는 생각으로 `panic` 전략을 성능 옵션(`lto`, `codegen-units`)과 같은 층위로 취급했을 때
- Tauri 앱 개발 초기에 CLI 도구의 `Cargo.toml`을 복사해 시작하면서 `panic = "abort"`를 그대로 물려받았을 때

**권장**:
```toml
# workspace root Cargo.toml — 공통 최적화 옵션만
[profile.release]
lto = true
codegen-units = 1
# panic 전략은 여기서 지정하지 않음(기본값 unwind 유지)

# cli/Cargo.toml — CLI 바이너리 크레이트 전용 override
[profile.release]
inherits = "release"
panic = "abort"   # 원샷 프로세스: 종료가 곧 실패 신호이므로 abort로 크기/속도 이득

# src-tauri/Cargo.toml — 데스크톱 앱: unwind(기본값) 유지, 명시하지 않음
```
- Cargo는 바이너리 크레이트별로 `[profile.release]`를 override할 수 있으므로(`inherits` 활용), workspace 공통 설정과 바이너리별 panic 전략을 분리한다.
- Tauri 앱 크레이트는 `panic`을 지정하지 않아 기본값 `unwind`를 유지하고, 워커 스레드 경계마다 `catch_unwind`(ERR-011)로 개별 실패를 흡수한다.
- CLI 크레이트는 `abort`를 선택하되, 그 이유(프로세스 종료 = 실패 신호, unwind 비용 불필요)를 주석으로 남겨 향후 누군가 "왜 이것만 다르지?"라고 묻지 않게 한다.

**탐지 방법**:
- 워크스페이스의 모든 `Cargo.toml`(root + 각 바이너리 크레이트)에서 `panic =` 설정을 수집해 표로 정리하고, 각 바이너리의 실행 모델(원샷 CLI vs 상시 구동 데스크톱 앱)과 일치하는지 대조
- Tauri 앱을 실제로 빌드해 워커 스레드에서 panic을 유발하는 통합 테스트를 돌렸을 때, 프로세스 전체가 죽는지 해당 요청만 실패하는지 관찰(전자라면 `abort`가 잘못 상속된 것)
- CI에 "release profile의 panic 전략이 바이너리 타입과 일치하는지" 확인하는 스크립트 추가(예: `src-tauri/Cargo.toml`에 `panic = "abort"`가 있으면 실패)

**예외**:
- 데스크톱 앱이라도 "패닉이 발생하면 상태 정합성을 신뢰할 수 없으니 차라리 즉시 종료 후 재시작하는 것이 낫다"는 명시적 제품 결정을 내린 경우 — 단, 이 경우 세션 상태를 주기적으로 자동 저장하는 보완책이 함께 있어야 사용자 경험이 허용 가능한 수준이 된다
- 라이브러리 크레이트에는 애초에 `panic` profile 키가 적용되지 않으므로(최종 바이너리/cdylib 링크 시점에만 유효) 이 항목은 해당하지 않는다

**Bitvue 판정**: Confirmed — **재감사(2026-08-18), 대상 자체가 바뀜**: 이전 판정이 우회 근거로 삼은 "Tauri 데스크톱 앱은 `exclude`로 제외돼 영향 밖"이라는 구조는 사라졌다 — `src-tauri`가 완전히 삭제됐고(`exclude = ["fuzz"]`만 남음, Cargo.toml:3), 데스크톱 앱은 이제 Electron(`bitvue-desktop/`, Node.js 프로세스, Rust `panic` 설정과 무관)이 담당한다. 루트 `Cargo.toml:131`은 여전히 `[profile.release]`에 `panic = "abort"`를 워크스페이스 전역으로 지정하며, 이는 원샷 CLI(`bitvue-cli`)뿐 아니라 **지금은 워크스페이스 멤버가 된 상시 구동 프로세스 두 개**(`bitvue-sidecar` — Electron이 자식 프로세스로 spawn해 데이터/렌더링 요청을 계속 처리하는 엔진, `bitvue-mcp` MCP 서버)에도 동일하게 적용된다. `bitvue-sidecar`는 이 카탈로그가 경고하는 문제(하나의 요청 처리 중 panic이 서버 프로세스 전체를 abort)를 그대로 안고 있지만, **완전히 무방비는 아니다** — `bitvue-desktop/src/sidecarClient.ts`가 sidecar의 예기치 않은 종료를 감지해 프로세스를 자동 재시작하는 워치독을 이미 구현하고 있다(`sidecar.on("exit", ...)`, 재시도 실패 시 `restart_failed` 이벤트). 다만 재시작 시 진행 중이던 스트림/세션 상태는 재생 없이 유실된다고 코드 주석에 명시돼 있어("nothing to replay them from"), 이 카탈로그가 우려하는 "세션 상태 전체 소실"이 여전히 실제로 발생한다 — 다만 그 반경이 "Electron 앱 전체 즉사"가 아니라 "sidecar 프로세스 재시작 + 열려있던 스트림 재오픈 필요"로 축소됐다는 점이 Tauri 시절과의 핵심 차이다.

---

### ERR-017: 위험한 FFI 디코더 호출에 프로세스/스레드 격리 없음
**분류**: ERR · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[tauri::command]
fn decode_frame_preview(path: String, frame_index: u32) -> Result<Vec<u8>, String> {
    // 메인 프로세스, 메인 이벤트 루프와 같은 주소 공간에서 서드파티 C 디코더를 직접 호출
    unsafe {
        let ctx = third_party_decoder_open(path.as_ptr());
        let frame = third_party_decoder_decode(ctx, frame_index);
        third_party_decoder_close(ctx);
        Ok(frame_to_png(frame))
    }
}
```

**문제**:
- 서드파티 C/C++ 디코더는 Rust의 안전성 보장 밖에 있다. 버퍼 오버런, use-after-free, 손상된 입력에 대한 어설션 실패(`abort()` 호출) 등은 Rust의 `Result`/`panic`/`catch_unwind` 어느 것으로도 잡을 수 없다 — 이들은 Rust 런타임을 거치지 않고 프로세스를 직접 죽인다.
- 이런 호출을 메인 Tauri 프로세스 안에서 직접 실행하면, 악의적이거나 단순히 버그가 있는 하나의 파일이 앱 전체 프로세스를 세그폴트/abort로 끝장낸다 — Rust 쪽에서 아무리 에러 처리를 잘해도 이 계층에서는 무력하다.
- 특히 Bitvue처럼 "신뢰할 수 없는 임의 파일을 여는" 도구에서 검증되지 않은 디코더 라이브러리(특히 실험적/커뮤니티 구현체)를 메인 프로세스에 직접 링크하는 것은 공격 표면을 그대로 앱 전체로 확장하는 것과 같다.

**발생 조건**:
- 신뢰도가 검증되지 않았거나 fuzzing 이력이 부족한 서드파티 디코더 라이브러리를 사용할 때
- 성능/구현 편의를 이유로 "일단 in-process로 붙이고 나중에 격리하자"는 임시방편이 그대로 굳어질 때
- 하드웨어 가속 디코더처럼 드라이버 레벨 크래시 가능성이 있는 경로를 메인 프로세스와 분리하지 않았을 때

**권장**:
```rust
// 별도 서브프로세스(또는 최소한 별도 OS 스레드 + 강한 리소스/시간 제한)에서 디코더 실행
fn decode_frame_preview(path: String, frame_index: u32) -> Result<Vec<u8>, AppError> {
    let child = std::process::Command::new(decoder_worker_binary_path())
        .arg("--path").arg(&path)
        .arg("--frame").arg(frame_index.to_string())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(AppError::WorkerSpawnFailed)?;

    let output = wait_with_timeout(child, Duration::from_secs(10))
        .map_err(|_| AppError::WorkerTimeout)?;

    if !output.status.success() {
        // 서브프로세스가 세그폴트/abort로 죽어도 시그널 정보가 여기 남고
        // 메인 프로세스는 영향받지 않음
        return Err(AppError::DecoderWorkerCrashed {
            exit_status: output.status,
            path: path.clone(),
            frame_index,
        });
    }
    Ok(output.stdout)
}
```
- 신뢰도가 낮거나 크래시 이력이 있는 FFI 디코더는 별도 서브프로세스(IPC로 결과만 주고받는 워커 바이너리)로 격리해, 그 프로세스가 죽어도 메인 앱은 "디코딩 실패"라는 정상적인 에러 응답만 받게 한다.
- 서브프로세스 격리가 성능상 부담스러운 경로에는 최소한 타임아웃과 워치독을 두고, OS 레벨 크래시 시그널(SIGSEGV, SIGABRT)을 exit status로 감지해 에러로 승격시킨다.
- 어떤 디코더 경로가 격리 대상인지(신뢰도, fuzzing 커버리지, 과거 크래시 이력 기준)를 문서화하고, 새 디코더 통합 시 이 기준으로 격리 필요 여부를 판단하는 체크리스트를 둔다.

**탐지 방법**:
- `extern "C"` 및 서드파티 디코더 크레이트 호출부가 메인 프로세스/메인 스레드에서 직접 실행되는지, 별도 프로세스 경계가 있는지 아키텍처 다이어그램 대조
- 해당 디코더 라이브러리를 실제로 손상된/fuzz 생성 샘플로 호출해 프로세스 전체가 죽는지(별도 프로세스라면 워커만 죽고 메인은 생존하는지) 관찰
- 의존성 목록에서 각 서드파티 디코더 라이브러리의 fuzzing/보안 감사 이력을 조사해 격리 우선순위를 매김

**예외**:
- 디코더 라이브러리가 자체적으로 강력한 fuzzing 이력과 보안 감사를 거쳐 신뢰도가 높다고 팀이 판단한 경우(예: 널리 쓰이는 성숙한 오픈소스 디코더) 격리 비용 대비 이득이 낮을 수 있다 — 단, 이 판단 근거를 문서로 남겨야 한다
- 개발/디버그 빌드에서 크래시 재현을 위해 의도적으로 in-process 호출을 사용하는 경우(재현 편의가 격리보다 우선)

**Bitvue 판정**: Confirmed — **재감사(2026-08-18), 심각도 실질적으로 완화됨**: `crates/bitvue-decode/src/vvdec.rs`(vvdec)/`decoder.rs`(dav1d)/`ffmpeg.rs`(ffmpeg-next)는 여전히 `extern "C"` FFI로 서드파티 C 디코더를 in-process로 호출하고, 이들을 부르는 `crates/bitvue-sidecar/src/decode_bridge.rs` 안에서도 추가적인 프로세스/스레드 격리는 없다(`std::process::Command` 사용처는 `vvdec.rs:937`의 `pkg-config` 빌드 감지뿐, 이전과 동일). **다만 아키텍처 자체가 바뀌어 이 항목이 우려하는 최악의 결과(전체 GUI 프로세스 즉사)는 이미 구조적으로 상당 부분 해소돼 있다**: `bitvue-sidecar`는 Electron 메인 프로세스가 `bitvue-desktop/electron/main.ts`에서 별도 OS 프로세스로 spawn하며(`SidecarClient`, `sidecarBinaryPath`), 두 프로세스는 stdio 기반 wire 프로토콜로만 통신한다 — 즉 손상된 파일이 vvdec/dav1d의 세그폴트/abort를 유발해도 죽는 것은 sidecar 프로세스 하나뿐이고, Electron 메인+렌더러(윈도우 UI 자체)는 별도 프로세스라 생존하며 `sidecar.on("exit", ...)` 핸들러가 이를 감지해 자동 재시작한다. 이는 이 카탈로그의 "권장" 예시가 요구하는 프로세스 경계 격리를 (개별 디코드 호출 단위는 아니지만) 엔진 전체 단위로 이미 확보한 것이다. 남은 갭: sidecar 프로세스 하나가 모든 스트림/디코더 호출을 공유하므로 한 파일의 크래시가 그 순간 sidecar가 처리 중이던 다른 스트림의 작업까지 함께 앗아가며(ERR-016 참고, 세션 상태 유실), 디코더 호출 단위의 더 세밀한 격리(타임아웃 있는 자식 프로세스 등)는 없다.

---

### ERR-018: 오류 타입이 "손상된 파일" vs "미구현 기능" vs "Bitvue 버그"를 구분하지 못함
**분류**: ERR · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("failed to process file: {0}")]
    ProcessingFailed(String),
}

fn parse_av1_obu(data: &[u8]) -> Result<Obu, AppError> {
    if data.is_empty() {
        return Err(AppError::ProcessingFailed("empty OBU".into()));
    }
    let obu_type = (data[0] >> 3) & 0x0F;
    match obu_type {
        1 => parse_sequence_header(data),
        6 => parse_frame(data),
        15 => Err(AppError::ProcessingFailed("padding OBU not yet supported".into())), // 미구현
        _ if obu_type > 8 => Err(AppError::ProcessingFailed("reserved obu_type".into())), // 손상/미지
        _ => unreachable!("internal parser bug"), // 진짜 버그
    }
}
```

**문제**:
- 세 가지 근본적으로 다른 상황 — "이 파일이 실제로 손상/스펙 위반됨", "Bitvue가 아직 이 기능을 구현하지 않음", "Bitvue 코드 자체에 버그가 있음" — 이 모두 같은 `ProcessingFailed(String)`으로 뭉뚱그려지면, 사용자와 개발자 모두 잘못된 행동을 하게 된다.
- 사용자는 "미구현 기능" 에러를 보고 "내 파일이 손상됐나?"라고 오해해 파일을 재인코딩하거나 다른 도구를 찾아 헤맬 수 있다 — 실제로는 Bitvue의 로드맵 문제일 뿐인데.
- 반대로 "Bitvue 버그"(예: 정상 파일인데 파서 로직 실수로 실패)를 "손상된 파일"처럼 보고하면, 개발자에게 버그 리포트가 오지 않고 사용자는 조용히 도구를 신뢰하지 않게 된다.
- 이 구분이 없으면 원격 측정(telemetry)에서도 "실제 버그 발생률"과 "사용자가 지원되지 않는 파일을 열어본 빈도"를 분리 집계할 수 없어, 무엇을 우선 고쳐야 할지(파서 버그 수정 vs 기능 확장) 판단할 근거가 사라진다.

**발생 조건**:
- 에러 타입을 설계할 때 "실패의 원인"이 아니라 "실패가 발생한 함수"를 기준으로 variant를 나눴을 때(원인 대신 위치로 분류)
- 기능을 점진적으로 구현하면서 "아직 안 만든 부분"을 위한 전용 에러 카테고리를 처음부터 만들어두지 않고, 임시로 기존 실패 경로에 문자열만 채워 넣었을 때
- `unreachable!`/`unimplemented!`(ERR-002)와 실제 파싱 실패가 같은 반환 타입으로 뒤섞여, 호출자가 무엇이 "우리 잘못"이고 무엇이 "입력 문제"인지 코드만으로 구분할 수 없을 때

**권장**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// 입력 파일 자체가 스펙을 위반했거나 손상됨 — 사용자에게 "이 파일에 문제가 있다"고 안내
    #[error("malformed input: {0}")]
    MalformedInput(#[from] ParseError),

    /// 스펙상 유효하지만 Bitvue가 아직 지원하지 않는 기능 — 사용자에게 "이 기능은 준비 중"이라고 안내
    #[error("unsupported feature: {feature} (tracked in {tracking_ref})")]
    NotYetSupported { feature: &'static str, tracking_ref: &'static str },

    /// Bitvue 자체의 논리 오류로 추정 — 사용자에게 버그 리포트를 유도
    #[error("internal error (please report this): {0}")]
    InternalBug(String),
}

fn parse_av1_obu(data: &[u8]) -> Result<Obu, AppError> {
    if data.is_empty() {
        return Err(AppError::MalformedInput(ParseError::Truncated {
            field: "obu_header", byte_offset: 0, bit_offset: 0,
        }));
    }
    let obu_type = (data[0] >> 3) & 0x0F;
    match obu_type {
        1 => parse_sequence_header(data).map_err(AppError::MalformedInput),
        6 => parse_frame(data).map_err(AppError::MalformedInput),
        15 => Err(AppError::NotYetSupported {
            feature: "OBU_PADDING",
            tracking_ref: "BITVUE-142",
        }),
        16..=31 => Err(AppError::MalformedInput(ParseError::FieldOutOfRange {
            field: "obu_type", value: obu_type as i64, byte_offset: 0,
        })), // reserved 영역: 스펙상 정의되지 않았을 뿐 valid range 내
        _ => Err(AppError::InternalBug(format!("obu_type {obu_type} not handled by match"))),
    }
}
```
- 최상위 에러 타입을 원인 축(입력 문제 / 미구현 / 내부 버그)으로 먼저 나누고, 각 축 아래에 세부 variant를 둔다.
- `NotYetSupported`에는 항상 추적 참조(이슈 번호 등)를 포함시켜, 사용자가 "이미 알려진 제한사항"임을 확인하거나 개발자가 우선순위를 매길 수 있게 한다.
- UI는 세 카테고리에 각기 다른 문구/행동을 연결한다: 손상된 입력 → "파일 확인 안내", 미구현 → "지원 예정 기능 안내 + 추적 링크", 내부 버그 → "버그 리포트 제출 유도(+ 진단 정보 자동 첨부)".

**탐지 방법**:
- 최상위 `AppError`(또는 동등 타입)의 variant 목록을 검토해 "원인" 기준으로 최소 3개 카테고리(입력/미구현/내부버그)가 구분되어 있는지 확인
- 문자열 메시지만 있고 구조화된 variant가 없는 에러(`ProcessingFailed(String)`류)를 grep으로 찾아 리팩터링 후보로 표시
- 실제 사용자 대상 에러 메시지를 수집해, "이 파일이 문제인지 Bitvue가 문제인지"를 메시지만 보고 구분할 수 있는지 UX 리뷰

**예외**:
- 프로토타입/실험 단계 코드에서 아직 기능 경계가 확정되지 않아 세 카테고리 구분이 시기상조인 경우(단, 정식 기능으로 승격되기 전에는 반드시 구조화해야 함)
- 매우 지역적인 유틸리티 함수(예: 순수 산술 헬퍼)처럼애초에 "입력 문제"라는 개념이 성립하지 않는 경우

**Bitvue 판정**: Confirmed — **재감사(2026-08-18), 근거는 바뀌었지만 결론 동일**: `src-tauri/src/error.rs`는 삭제됐지만, 정확히 이 항목이 요구하는 구조화 타입이 이제 백엔드 쪽에 새로 존재한다 — `crates/bitvue-protocol/src/lib.rs`의 `WireErrorCode`(Parse/Decode/NotFound/Internal 등, ERR-003 참고)와 `crates/bitvue-engine/src/error.rs`의 `BitvueError`. 문제는 프런트엔드가 이를 전혀 소비하지 않는다는 점이다: `frontend/errors/appError.ts`의 `ErrorCategory`(Validation/NotFound/Permission/Parse/Io/Network/Codec/Internal/**NotImplemented** — 정확히 이 항목이 요구하는 "손상된 입력/미구현/내부버그" 3분류에 대응하는 카테고리가 이미 다 정의돼 있음, 18-70행)를 실제로 `ErrorCategory.X` 형태로 사용하는 non-test 프런트엔드 파일은 `appError.ts` 자기 자신 말고 전무하다(grep 무결과) — 즉 이 분류 타입은 정의만 되고 백엔드에서 넘어온 에러를 분류하는 데 배선된 적이 없는 죽은 taxonomy다. Rust 쪽 `WireErrorCode`가 프런트까지 도달하지 못하는 것도 ERR-003에서 확인한 그대로다.
**관련**: 배선 문제 관점은 `WIRING.md`의 WIRE-005 참고(재확인 필요 — 참조 대상이 옛 경로일 수 있음).

---

### ERR-019: 에러 체인(source)이 변환 과정에서 유실됨
**분류**: ERR · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Debug, thiserror::Error)]
#[error("failed to load sample file")]
pub struct LoadError; // 원인 정보를 전혀 담지 않는 유닛 struct

fn load_sample(path: &Path) -> Result<Bitstream, LoadError> {
    let data = std::fs::read(path).map_err(|_| LoadError)?;      // io::Error 버림
    let bitstream = parse_container(&data).map_err(|_| LoadError)?; // ContainerError 버림
    Ok(bitstream)
}
```

**문제**:
- `map_err(|_| LoadError)`는 하위 계층이 이미 만들어둔 구체적인 원인(어떤 IO 에러였는지, 어떤 컨테이너 파싱 실패였는지)을 그 자리에서 완전히 폐기한다. 이후 어떤 계층에서도 이 정보를 복구할 수 없다.
- Rust의 `std::error::Error::source()` 체인이 끊기면, `anyhow`/로깅 프레임워크가 자동으로 제공하는 "원인 체인 전체 출력" 기능(`{:#}` 포맷, `anyhow::Error::chain()` 등)이 무용지물이 된다 — 최상위 메시지 한 줄만 남는다.
- 버그 리포트에 "failed to load sample file"만 찍히면, 그것이 "파일이 존재하지 않음(NotFound)"인지 "권한 없음(PermissionDenied)"인지 "컨테이너 포맷이 아예 다름"인지 구분할 방법이 없어, 재현/진단에 훨씬 더 많은 왕복이 필요해진다.

**발생 조건**:
- 에러 타입을 "간단하게" 만들려고 원인 필드 없이 유닛 struct나 단순 문자열로 정의했을 때
- `map_err(|_| ...)`처럼 하위 에러 값을 아예 바인딩하지 않고 버리는 패턴을 습관적으로 사용할 때
- 여러 계층을 거치며 매번 새 에러 타입으로 감싸되, `#[source]`/`#[from]` 어노테이션을 빠뜨렸을 때

**권장**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to read file {path}")]
    Io { path: PathBuf, #[source] source: std::io::Error },

    #[error("failed to parse container")]
    Container(#[source] #[from] ContainerError),
}

fn load_sample(path: &Path) -> Result<Bitstream, LoadError> {
    let data = std::fs::read(path)
        .map_err(|source| LoadError::Io { path: path.to_path_buf(), source })?;
    let bitstream = parse_container(&data)?; // #[from]이 자동으로 소스를 보존하며 변환
    Ok(bitstream)
}
```
- `thiserror`의 `#[source]`(그리고 가능하면 `#[from]`)를 사용해 하위 에러 값을 항상 보존한다 — `map_err(|_| ...)`로 값을 버리지 않는다.
- 최상위 로깅/에러 리포팅 지점에서는 `error.source()`를 재귀적으로 순회하거나(`anyhow` 사용 시 `{:#}`/`chain()`) 원인 체인 전체를 기록한다.
- 클립피 린트(`clippy::result_map_or_into_option` 계열은 다르지만) 대신 코드 리뷰 관례로 "`map_err`의 클로저가 인자를 사용하지 않으면 원인 유실을 의심하라"를 체크리스트에 둔다.

**탐지 방법**:
- `grep -rn "map_err(|_|" src/` — 인자를 무시하는 `map_err` 클로저는 원인 유실의 강력한 신호
- 에러 타입 정의에서 `#[source]`/`#[from]` 없이 문자열이나 유닛 variant만 있는 곳을 리스트업
- 실제 실패를 재현했을 때 로그에 원인 체인(예: "caused by: ...")이 몇 단계까지 출력되는지 확인 — 항상 1단계뿐이면 어딘가에서 체인이 끊긴 것

**예외**:
- 원인 정보가 보안/개인정보상 노출되면 안 되는 극히 예외적인 경우(예: 파일 경로 자체가 민감 정보를 포함) — 이 경우도 완전 폐기보다는 내부 로그에는 보존하고 사용자 노출 메시지에서만 마스킹하는 것이 낫다
- 원인이 정말로 의미가 없는 경우(예: `Option::None`을 에러로 변환하는 지점처럼 애초에 하위 에러 객체가 존재하지 않는 경우)

**Bitvue 판정**: Confirmed — **재검증(2026-08-18)**: `bitvue-formats`/`bitvue-decode`는 마이그레이션과 무관해 인용된 지점이 그대로 재확인된다. 현재 워크스페이스 전체(테스트 제외)에서 `map_err(|_| ...)` 패턴은 60건으로 집계된다(이전 77건에서 소폭 감소 — 정확한 원인은 미조사, 다른 리팩터의 부수효과로 추정). `crates/bitvue-formats/src/mp4.rs:36,45,54,63`은 여전히 실제 `io`/파싱 실패를 `BitvueError::UnexpectedEof(cursor.position())`로 변환하며 원본 에러를 완전히 버리고, `crates/bitvue-decode/src/decoder.rs:841,848,881,894`도 IVF 헤더 필드 파싱 실패(`TryFromSliceError`)를 `DecodeError::Decode("IVF ... bytes invalid")` 문자열로 뭉개 원인을 유실한다(행 번호까지 이전 판정과 동일). 같은 저장소의 `HevcError`가 `#[from]`/`#[source]`로 원인을 보존하는 것과 대조적으로 일관성이 없다는 결론도 유효.

---

### ERR-020: 실패를 로그에만 남기고 사용자에게 알리지 않음
**분류**: ERR · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn load_thumbnail_cache(path: &Path) -> Option<ThumbnailCache> {
    match ThumbnailCache::load(path) {
        Ok(cache) => Some(cache),
        Err(e) => {
            log::warn!("failed to load thumbnail cache: {e}");
            None // 호출자는 "캐시가 원래 없었던 것"과 "캐시 로드가 실패한 것"을 구분 못함
        }
    }
}

fn parse_optional_sei(data: &[u8]) -> Vec<SeiMessage> {
    parse_sei_messages(data).unwrap_or_else(|e| {
        log::debug!("SEI parsing failed, ignoring: {e}"); // debug 레벨 -> 사실상 아무도 안 봄
        Vec::new()
    })
}
```

**문제**:
- `Option<T>`/기본값으로의 조용한 폴백은 "실패했다"는 정보를 함수 시그니처에서부터 지워버린다. 호출자는 로그를 뒤지지 않는 한 실패가 있었는지조차 알 수 없다.
- 두 번째 예시처럼 SEI(부가 정보) 같은 "있으면 좋지만 없어도 되는" 데이터의 파싱 실패를 조용히 삼키는 것은 개별적으로는 합리적일 수 있지만, 이런 패턴이 코드베이스 전반에 원칙 없이 흩어져 있으면 "지금 화면에 보이는 정보가 실제로 없는 것인지, 파싱에 실패해서 빠진 것인지"를 사용자가 영영 구분할 수 없게 된다.
- 로그 레벨을 `debug`로 낮춰두면 프로덕션 빌드/일반 사용자 환경에서는 사실상 아무 데도 기록되지 않아, 개발자조차 이 실패의 발생 빈도를 파악할 방법이 없다.

**발생 조건**:
- "이 필드는 optional이니 실패해도 무해하다"고 판단해 에러를 삼키되, 그 판단 근거를 UI에 전혀 반영하지 않았을 때
- 캐시/프리페치처럼 "실패해도 재계산하면 그만"인 경로에서 실패를 로그로만 남기는 관행이, 사용자가 결과에 영향을 미치는 경로까지 무분별하게 확산됐을 때
- 로그 레벨 설정(release 빌드의 기본 로그 레벨)을 고려하지 않고 개발 중 편의를 위해 낮은 레벨로 기록해둔 채 방치했을 때

**권장**:
```rust
enum ThumbnailCacheLoad {
    Loaded(ThumbnailCache),
    NotFound,               // 캐시가 원래 없음(정상)
    LoadFailed(CacheError), // 있었는데 손상/읽기 실패(비정상 — UI에 표시 가치 있음)
}

fn load_thumbnail_cache(path: &Path) -> ThumbnailCacheLoad {
    if !path.exists() {
        return ThumbnailCacheLoad::NotFound;
    }
    match ThumbnailCache::load(path) {
        Ok(cache) => ThumbnailCacheLoad::Loaded(cache),
        Err(e) => {
            log::warn!("thumbnail cache at {path:?} exists but failed to load: {e}");
            ThumbnailCacheLoad::LoadFailed(e) // 호출자가 "재생성할지, 사용자에게 알릴지" 선택 가능
        }
    }
}

// SEI처럼 진짜 optional인 정보도, 실패했다는 사실 자체는 결과에 남긴다
struct FrameAnalysis {
    sei_messages: Vec<SeiMessage>,
    sei_parse_warning: Option<String>, // 없으면 "정상적으로 없음", Some이면 "파싱 실패로 누락"
}
```
- "값이 원래 없음"과 "값을 얻으려 했지만 실패함"을 같은 `None`/빈 컬렉션으로 뭉개지 말고, 최소한 결과 구조체에 실패 여부를 남길 수 있는 필드를 둔다.
- UI 정책을 명시적으로 정한다: 어떤 실패는 사용자에게 즉시 노출(모달/토스트), 어떤 실패는 "정보" 아이콘 하나로 은근히 노출, 어떤 실패는 정말로 로그만으로 충분한지 — 이 경계를 팀 컨벤션으로 문서화한다.
- 사용자에게 영향을 주는 실패(썸네일 누락, 부가 정보 누락 등)는 최소 `warn` 레벨로 기록해 release 빌드에서도 로그 수집 시 확인 가능하게 한다.

**탐지 방법**:
- `unwrap_or_else`/`unwrap_or_default`/`.ok()` 뒤에 로그만 있고 반환 타입이 `Option`/빈 컬렉션으로 축소되는 패턴을 grep으로 탐색
- 각 축소 지점에 대해 "이 실패가 최종적으로 화면에 어떤 형태로든 흔적을 남기는가?"를 수동으로 추적(대부분 남기지 않으면 이 항목의 위반)
- 로그 레벨 설정(release 기본값)을 확인하고, `debug`/`trace`로 기록된 실패 중 사용자 가시 결과에 영향을 주는 것이 있는지 점검

**예외**:
- 정말로 부가적이고 사용자가 그 존재 여부를 인지할 필요가 없는 내부 최적화 경로(예: 캐시 워밍 실패, 백그라운드 prefetch 실패로 인한 단순 재계산)는 로그만으로 충분하다
- 반복적으로 매우 자주 발생하며 개별 알림이 오히려 사용자 경험을 해치는 경우(예: 초당 수십 번 호출되는 경로) — 이 경우 집계된 요약(예: "이번 세션에서 N개 항목 로드 실패")으로 대체하는 것이 낫다

**Bitvue 판정**: Suspected — **재감사(2026-08-18)**: `src-tauri`는 삭제됐으므로 현재 경로(`crates/bitvue-sidecar/src`, `crates/bitvue-engine/src`)로 다시 훑었다. ERR-008 재감사에서 이미 확인했듯, `bitvue-sidecar`의 여러 AV1 분석 핸들러가 쓰는 `ObuIterator` 순회/`overlay_extraction/parser.rs`의 resilient 폴백은 개별 OBU 파싱 실패를 `filter_map(|r| r.ok())` 등으로 조용히 건너뛰며, 이 실패가 결과 구조체("이 프레임은 정상 분석됨" vs "일부 OBU 스킵됨")에 흔적을 남기는지, 그리고 프런트가 이를 구분해 표시하는지는 확인하지 못했다 — 이 항목이 정확히 지적하는 "값이 원래 없음"과 "실패해서 없음"의 구분 불가 사례에 해당할 가능성이 있으나 UI 도달 여부까지 추적하지 못해 Suspected로 남긴다.

---
