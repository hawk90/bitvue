# Anti-Pattern Catalog — FFI: Rust ↔ C/C++ FFI

이 문서는 Bitvue(Tauri + Rust + React 기반 비디오 비트스트림 분석기) 안티패턴 카탈로그의 일부입니다(Phase 4, 시스템/인프라 웨이브). 전체 목록과 카테고리 구성은 `docs/anti-patterns/INDEX.md`를 참고하십시오. 본 카테고리는 성능이 아니라 **UB(undefined behavior)와 크래시 위험**을 다루는 카테고리로, 다른 카테고리보다 심각도를 보수적으로(더 높게) 매깁니다. Bitvue는 AV1 디코딩에 dav1d, 품질 지표 계산에 libvmaf, 그 외 코덱 지원을 위해 FFmpeg 계열 디코더 바인딩을 링크할 가능성이 있는 C 라이브러리 소비 프로젝트이며, 이 문서의 모든 예시는 이 세 라이브러리군(dav1d의 `Dav1dPicture`/`Dav1dContext`, libvmaf의 `VmafContext`/`VmafPicture`, FFmpeg류의 `AVFrame`/`AVPacket`)을 구체적 근거로 삼는다. 1단계(본 문서)는 일반적·도메인 특화 참조 카탈로그이며, 실제 저장소에 대한 감사와 판정은 2단계에서 수행합니다.

---

### FFI-001: C pointer를 Rust reference로 너무 일찍 변환
**분류**: FFI · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
// dav1d가 채워준 picture 포인터를 받아 곧바로 &Dav1dPicture로 역참조
unsafe extern "C" fn on_frame_ready(pic: *mut Dav1dPicture, user_data: *mut c_void) {
    let pic: &Dav1dPicture = &*pic; // null/정렬/유효성 미검증 상태에서 즉시 reference화
    let width = pic.p.w;
    forward_to_pipeline(width, pic.data[0]);
}
```

**문제**:
- Rust reference는 "항상 유효하고, 정렬되어 있고, non-null"이라는 컴파일러 차원의 계약을 지므로, 이 계약을 검증하지 않은 raw pointer를 reference로 바꾸는 순간 이미 UB가 발생할 수 있음(실제로 크래시가 나기 전이라도)
- 이후 이 reference를 함수 인자로 넘기면 컴파일러가 "이 값은 항상 유효하다"고 가정해 최적화를 수행 → null이었거나 정렬이 깨졌던 경우 디버그 빌드에서는 우연히 동작하다가 release 빌드에서만 크래시
- 역참조 시점과 실제 사용 시점이 분리되어, 문제가 되는 라인과 크래시가 나는 라인이 다른 함수/다른 스레드로 흩어짐

**발생 조건**:
- C 콜백 시그니처를 그대로 받아 함수 최상단에서 습관적으로 `&*ptr` 또는 `&mut *ptr`을 수행할 때
- "이 콜백은 항상 유효한 포인터로 호출된다"는 라이브러리 문서를 신뢰하고 방어 코드를 생략할 때

**권장**:
```rust
unsafe extern "C" fn on_frame_ready(pic: *mut Dav1dPicture, user_data: *mut c_void) {
    let Some(pic) = NonNull::new(pic) else {
        log::error!("dav1d callback invoked with null picture pointer");
        return;
    };
    // 필요한 필드만 raw pointer 연산으로 읽고, reference는 실제 소비 직전에만 생성
    let width = unsafe { (*pic.as_ptr()).p.w };
    // ...
}
```
- raw pointer 상태에서 null/정렬 검증을 마친 뒤 가장 좁은 스코프에서만 reference를 만든다
- `ptr::read_unaligned` 등 정렬을 보장할 수 없는 경우 전용 API를 사용한다
- 콜백 최상단에서 무조건 reference화하는 패턴을 코드 리뷰 체크리스트에 명시적으로 금지

**탐지 방법**:
- Miri로 FFI 콜백 진입 경로를 포함한 테스트 실행(정렬/유효성 위반을 상당수 포착)
- 구조적 검사: 함수 시작 직후 몇 줄 이내에 `&*` 또는 `&mut *`가 나타나는 unsafe 함수를 grep으로 추출해 리뷰
- 코드 리뷰에서 "이 reference가 만들어지는 시점에 무엇이 검증되어 있는가"를 질문

**예외**:
- 라이브러리가 ABI 문서에서 "이 콜백은 절대 null로 호출되지 않는다"를 명시하고, 해당 보장이 fuzzing/CI로 지속 검증되는 경우에도 최소한 `debug_assert!`는 남기는 것이 바람직

**Bitvue 판정**: N/A — 저장소 내 유일한 raw picture pointer 역참조는 `crates/bitvue-decode/src/vvdec.rs:378`의 `let vf = &*frame;`이며, 직전 `frame.is_null()` 체크(vvdec.rs:374)를 통과한 뒤에만 실행됨. dav1d/FFmpeg 경로는 safe wrapper crate(`dav1d`, `ffmpeg-next`)의 안전 API(`Picture::plane()` 등, decoder.rs/ffmpeg.rs)만 사용해 raw pointer 역참조 자체가 없음

---

### FFI-002: nullable pointer를 non-null로 가정
**분류**: FFI · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
fn decode_frame(ctx: *mut Dav1dContext, data: &[u8]) -> Dav1dPicture {
    unsafe {
        let mut pic: Dav1dPicture = mem::zeroed();
        dav1d_get_picture(ctx, &mut pic); // 실패 시 pic 내부 포인터가 null일 수 있음
        // 반환값(에러 코드)을 확인하지 않고 바로 사용
        pic
    }
}

fn frame_luma_ptr(pic: &Dav1dPicture) -> *const u8 {
    pic.data[0] // EAGAIN/EOF 상황에서 null일 수 있는데 그대로 반환해 나중에 역참조
}
```

**문제**:
- dav1d, libvmaf, FFmpeg 계열 API는 "성공 시 유효 포인터, 실패/버퍼링 상태 시 null 또는 특정 에러코드"라는 관례를 광범위하게 사용하는데, 이를 무시하고 항상 성공했다고 가정
- null 역참조는 대부분의 플랫폼에서 세그폴트로 끝나지만, 항상 그런 것은 아니며(예: 0 근처 매핑된 페이지가 있는 임베디드/특수 환경) UB로서 더 나쁜 결과(조용한 데이터 손상)도 이론상 가능
- 에러 코드를 확인하지 않으면 "디코딩 실패"와 "프레임 없음(EAGAIN)"과 "정상 종료(EOF)"를 구분하지 못해 상위 로직이 잘못된 상태로 진행

**발생 조건**:
- `dav1d_get_picture`, `dav1d_send_data`처럼 반환값이 있는 함수를 호출하고 반환값을 버릴 때
- 성공 경로만 로컬에서 테스트하고, 스트림 끝/디코더 warm-up 구간(초기 몇 프레임은 아직 출력이 없음) 같은 엣지 케이스를 검증하지 않을 때

**권장**:
```rust
fn decode_frame(ctx: *mut Dav1dContext) -> Result<Dav1dPicture, DecodeError> {
    unsafe {
        let mut pic: Dav1dPicture = mem::zeroed();
        let ret = dav1d_get_picture(ctx, &mut pic);
        match ret {
            0 => Ok(pic),
            e if e == -EAGAIN => Err(DecodeError::NeedMoreData),
            e => Err(DecodeError::Native(e)),
        }
    }
}
```
- 모든 C 함수 반환값(에러코드)을 `#[must_use]`로 취급하고 명시적으로 매칭한다
- 포인터를 반환하는 필드는 `Option<NonNull<T>>`로 감싸는 안전 래퍼를 한 겹 둔다
- `#[must_use]`가 적용된 자체 wrapper 타입을 만들어 반환값 무시를 컴파일 경고로 승격

**탐지 방법**:
- clippy `unused_must_use`, 자체 바인딩에 `#[must_use]` 부여 후 정적 검사
- 구조적 검사: 반환값을 받지 않는(`let _ =` 또는 아예 무시) C 함수 호출 grep
- Runtime: EAGAIN/EOF 경로를 강제로 유도하는 통합 테스트(짧은 스트림, 손상된 비트스트림) 추가

**예외**:
- 함수 시그니처 자체가 절대 실패하지 않음을 문서로 보장하는 단순 getter류(예: 상수 반환)는 예외로 둘 수 있으나, 이 경우도 주석으로 근거를 남긴다

**Bitvue 판정**: N/A — vvdec 호출부는 반환값/null을 매번 체크: `vvdec_decoder_open` null 체크(vvdec.rs:334), `vvdec_accessUnit_alloc` null 체크(vvdec.rs:345), `vvdec_accessUnit_alloc_payload` ret 체크(vvdec.rs:602), `vvdec_decode`/`vvdec_flush` 결과는 `ret == VVDEC_OK`로 분기 후에만 frame_ptr 사용(vvdec.rs:710-717). dav1d/libvmaf-rs는 wrapper crate가 `Result`로 이미 강제

---

### FFI-003: C buffer lifetime보다 Rust slice가 오래 생존
**분류**: FFI · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
fn luma_plane<'a>(pic: &'a Dav1dPicture) -> &'a [u8] {
    unsafe {
        let len = (pic.stride[0] as usize) * (pic.p.h as usize);
        slice::from_raw_parts(pic.data[0] as *const u8, len)
    }
}

fn analyze(ctx: *mut Dav1dContext) {
    let pic = get_next_picture(ctx);
    let plane = luma_plane(&pic);
    dav1d_picture_unref(&mut pic as *mut _); // 여기서 buffer 반환/재사용됨
    compute_histogram(plane); // plane은 이미 해제되었거나 다른 프레임이 덮어쓴 메모리
}
```

**문제**:
- `slice::from_raw_parts`로 만든 slice의 Rust 수명(`'a`)은 컴파일러가 검증해 주지 않는다 — 함수 시그니처에 어떤 lifetime을 붙이든 실제 유효 구간은 C 라이브러리의 내부 refcount/pool 재사용 정책이 결정한다
- dav1d의 picture buffer는 내부 buffer pool에서 재사용되므로 `dav1d_picture_unref` 이후 즉시 다른 디코딩 작업에 재할당될 수 있음 → use-after-free가 아니라 "다른 프레임 데이터를 읽는" 조용한 데이터 오염으로 나타날 수도 있어 탐지가 더 어려움
- 컴파일러가 borrow checker로 걸러주지 못하는 유일한 안전 계층이 "개발자의 기억"이 되어버림

**발생 조건**:
- 디코더가 반환한 picture의 raw 데이터 포인터로 slice를 만든 뒤, `unref`/`free` 호출 전에 slice 사용을 끝내지 않는 모든 경로(비동기 큐잉, 다른 스레드로 전달, 콜백 체이닝)
- 특히 slice를 별도 채널/큐에 넣어 다른 스레드에서 나중에 소비하는 파이프라인 구조

**권장**:
```rust
struct DecodedFrame {
    pic: Dav1dPicture, // picture 소유권(및 내부 refcount)을 함께 보관
}

impl DecodedFrame {
    fn luma_plane(&self) -> &[u8] {
        unsafe {
            let len = (self.pic.stride[0] as usize) * (self.pic.p.h as usize);
            slice::from_raw_parts(self.pic.data[0] as *const u8, len)
        }
    }
}

impl Drop for DecodedFrame {
    fn drop(&mut self) {
        unsafe { dav1d_picture_unref(&mut self.pic) };
    }
}
```
- raw slice를 넘기지 말고, unref를 책임지는 소유권 래퍼(`DecodedFrame`)를 넘겨서 slice의 실제 수명을 wrapper의 `Drop`에 강제로 연결한다
- 파이프라인 경계를 넘길 필요가 있다면 slice가 아니라 `Arc<DecodedFrame>`처럼 refcount를 공유하는 소유권 자체를 넘긴다
- 정말 복사가 필요하면 `to_vec()`으로 명시적 소유 버퍼를 만든다(성능 트레이드오프를 주석에 남김)

**탐지 방법**:
- Miri + ASan(`-Zsanitizer=address`)으로 use-after-free 및 buffer overrun 감지 테스트
- 구조적 검사: `slice::from_raw_parts` 호출 결과가 함수 반환값이거나 unref/free 호출 이전에 소비되지 않는 경로를 리뷰
- fuzzing: 짧은 GOP로 빠르게 디코딩→해제를 반복시켜 pool 재사용 타이밍을 인위적으로 앞당기는 스트레스 테스트

**예외**:
- slice의 소비가 unref 호출 이전, 같은 함수 스코프 내에서 완전히 끝나고 다른 스레드로 넘어가지 않는 경우는 정적으로도 안전을 논증하기 쉬움(단, 이 경우도 명시적 주석 권장)

**Bitvue 판정**: N/A — `extract_plane`이 만든 raw slice(vvdec.rs:546)는 decoder+access_unit 뮤텍스가 잡힌 채로 즉시 `plane_utils::extract_plane`을 통해 소유 `Vec<u8>`로 복사된 뒤에만 반환됨. vvdec.rs:508-545에 이 불변조건을 명시한 상세 SAFETY 주석 존재

---

### FFI-004: decoder-owned frame을 해제 후 참조
**분류**: FFI · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
struct FrameCache {
    last_frame: Option<*mut Dav1dPicture>,
}

impl FrameCache {
    fn update(&mut self, ctx: *mut Dav1dContext) {
        let mut pic: Dav1dPicture = unsafe { mem::zeroed() };
        unsafe { dav1d_get_picture(ctx, &mut pic) };
        self.last_frame = Some(&mut pic as *mut _); // pic은 함수 종료 시 스택에서 소멸
    }

    fn diff_with_previous(&self) {
        if let Some(ptr) = self.last_frame {
            unsafe { println!("{}", (*ptr).p.w) }; // dangling pointer 역참조
        }
    }
}
```

**문제**:
- 스택에 있던 `Dav1dPicture`의 주소를 저장해 함수 스코프 밖에서 사용 — 전형적인 dangling pointer이며, FFI-003과 달리 이번엔 C 라이브러리가 아니라 Rust 쪽 스택 프레임이 소멸 원인
- 더 흔한 변형은 `dav1d_picture_unref`를 이미 호출한 뒤에도 캐시에 저장해둔 포인터를 계속 들고 있다가 다음 프레임 비교(`diff`), 오버레이 렌더링, 통계 집계 등에서 재사용하는 경우
- release 빌드에서는 해제된 메모리가 재사용되기 전까지 우연히 유효한 값처럼 보여서 로컬 테스트를 통과하고, 프로덕션 부하(프레임 처리량 증가)에서만 재현되는 경우가 많음

**발생 조건**:
- "직전 프레임과 비교"처럼 프레임 간 상태를 유지해야 하는 기능(scene change 검출, temporal 오버레이)에서 이전 프레임 buffer를 캐시할 때
- unref 호출 위치와 캐시 무효화 위치가 서로 다른 함수/모듈에 있어 한쪽만 수정되고 다른 쪽이 누락될 때

**권장**:
```rust
struct FrameCache {
    last_frame: Option<DecodedFrame>, // 소유권 있는 wrapper (FFI-003 참고)
}

impl FrameCache {
    fn update(&mut self, frame: DecodedFrame) {
        self.last_frame = Some(frame); // 이전 값은 Drop되며 안전하게 unref됨
    }

    fn diff_with_previous(&self, current: &DecodedFrame) -> Option<Diff> {
        self.last_frame.as_ref().map(|prev| compute_diff(prev, current))
    }
}
```
- 프레임 간 상태를 유지해야 한다면 raw pointer가 아니라 소유권 있는 래퍼 값을 저장해 Rust의 `Drop`/이동 의미에 해제 타이밍을 맡긴다
- unref와 캐시 무효화를 반드시 같은 타입의 `Drop` 구현 한 곳에 모아 "따로 관리되는 두 개의 해제 시점"을 없앤다

**탐지 방법**:
- 구조적 검사: 구조체 필드에 `*mut`/`*const` raw pointer를 저장하는 모든 곳을 목록화해, 그 포인터의 발급처가 스택 지역 변수인지 heap/refcounted 객체인지 확인
- Miri, ASan으로 캐시 갱신 → 이전 프레임 접근 시나리오를 포함한 테스트
- 코드 리뷰: "이 포인터가 가리키는 객체는 누가, 언제 해제하는가"를 필드 단위로 질문

**예외**:
- 없음 — 해제된 decoder-owned 객체에 대한 dangling reference 보관은 항상 버그로 취급한다

**Bitvue 판정**: N/A — 이전 프레임의 raw pointer를 구조체 필드에 캐시하는 코드가 없음(grep 결과 없음); `convert_frame` 직후 성공/실패 무관하게 항상 `vvdec_frame_unref` 호출(vvdec.rs:717)

---

### FFI-005: stride를 무시하고 연속 메모리로 가정
**분류**: FFI · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn copy_luma(pic: &Dav1dPicture, out: &mut Vec<u8>) {
    let w = pic.p.w as usize;
    let h = pic.p.h as usize;
    unsafe {
        // width * height가 곧 buffer 크기라고 가정 — stride(padding)를 무시
        let data = slice::from_raw_parts(pic.data[0] as *const u8, w * h);
        out.extend_from_slice(data);
    }
}
```

**문제**:
- dav1d/FFmpeg 계열 디코더는 SIMD 정렬, 서브샘플링, 크롭 등을 이유로 각 row 사이에 padding을 두는 stride(`linesize`) 기반 메모리 레이아웃을 사용하며, `stride >= width * bytes_per_pixel`이 일반적이고 종종 상당히 크다(예: 64/128바이트 정렬)
- `width * height`만큼 읽으면 실제 이미지보다 적게 읽거나(스트라이드가 큰 경우 row 경계가 밀려 다음 row의 앞부분을 잘못 잘라 읽음), 반대로 buffer 끝을 넘어 읽어(OOB read) UB가 됨
- 결과물이 "완전히 깨진 이미지"가 아니라 "약간 기울어지거나 노이즈가 낀 이미지"로 보여서, 렌더링 버그로 오인하고 잘못된 곳을 디버깅하게 되는 경우가 많음

**발생 조건**:
- 해상도가 stride 정렬 경계(예: 64의 배수)와 딱 맞아떨어지는 테스트 영상으로만 검증해 stride == width인 우연한 케이스만 통과했을 때
- 크롭된 해상도(예: 1920x1080이지만 내부 버퍼는 1920x1088로 정렬)를 다루는 코덱에서 특히 잘 드러남

**권장**:
```rust
fn copy_luma(pic: &Dav1dPicture, out: &mut Vec<u8>) {
    let w = pic.p.w as usize;
    let h = pic.p.h as usize;
    let stride = pic.stride[0] as usize;
    out.reserve(w * h);
    unsafe {
        let base = pic.data[0] as *const u8;
        for row in 0..h {
            let row_ptr = base.add(row * stride);
            out.extend_from_slice(slice::from_raw_parts(row_ptr, w));
        }
    }
}
```
- row 단위로 stride만큼 건너뛰며 필요한 width만큼만 읽는다(row-copy 패턴)
- 가능하면 이 로직을 한 곳(plane accessor 유틸리티)에 모아 매번 stride 산술을 재작성하지 않게 한다
- 비정상 해상도(정렬 경계와 딱 맞지 않는 해상도)를 포함한 테스트 스트림을 CI에 반드시 포함

**탐지 방법**:
- 구조적 검사: `pic.data[0]`/`data[1]` 등에서 `from_raw_parts`를 만들며 `stride` 필드를 전혀 참조하지 않는 함수를 grep
- Runtime: 비정렬 해상도(예: 1919x1079처럼 홀수/비정렬 크기) 테스트 스트림으로 골든 이미지 비교 회귀 테스트
- 시각적 회귀: 디코딩 결과 이미지의 대각선 노이즈/기울어짐 패턴을 자동 검출하는 스모크 테스트

**예외**:
- 이미 stride == width가 보장된 API(예: 라이브러리가 명시적으로 "output은 항상 tightly packed"라고 문서화한 특정 output 모드)를 사용한다면 예외이나, 이 가정 자체를 `debug_assert!(stride == width * bpp)`로 코드에 박아둘 것

**Bitvue 판정**: N/A — `plane_utils::extract_plane`(plane_utils.rs:223-287)이 이미 stride를 인지한 row-by-row 복사(contiguous fast-path + strided fallback)를 구현하고 있어 권장안과 동일한 패턴이 적용되어 있음; vvdec.rs/ffmpeg.rs 양쪽 모두 이 유틸리티를 통해서만 plane을 추출함

---

### FFI-006: negative stride 미처리
**분류**: FFI · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn row_ptr(pic: &Dav1dPicture, row: usize) -> *const u8 {
    let stride = pic.stride[0]; // i32, 이 값이 음수일 수 있음을 가정하지 않음
    unsafe { (pic.data[0] as *const u8).add(row * stride as usize) }
}
```

**문제**:
- 일부 C 라이브러리/컨테이너 관례(특히 BMP 유래 bottom-up 레이아웃을 다루는 브리지 코드나, 특정 FFmpeg 필터 체인 출력)는 stride를 음수로 표현해 "데이터가 마지막 row부터 시작해 위로 올라간다"는 것을 나타낸다
- `stride as usize`로 캐스팅하면 음수 값이 거대한 양수로 wrap되어(`-8` → `usize::MAX - 7`) 포인터 산술이 사실상 임의의 메모리 주소를 가리키게 되고, 이는 명백한 OOB access UB
- 이 문제는 대부분의 정상 스트림/정상 디코더 경로에서는 나타나지 않고(양수 stride만 관찰됨) 특정 필터·변환 경로에서만 촉발되어 회귀 테스트 커버리지에서 빠지기 쉽다

**발생 조건**:
- dav1d 자체보다는, dav1d 출력을 다른 C 라이브러리(색공간 변환기, 특정 렌더러)로 넘기는 브리지 계층에서 그 라이브러리가 negative stride 관례를 쓸 때
- libvmaf에 외부에서 만든 picture buffer를 공급할 때, 공급원이 top-down이 아닌 bottom-up 레이아웃을 줄 때

**권장**:
```rust
fn row_ptr(pic: &Dav1dPicture, row: usize) -> *const u8 {
    let stride = pic.stride[0] as isize; // 부호 유지
    let base = pic.data[0] as *const u8;
    unsafe { base.offset(row as isize * stride) } // offset은 음수 오프셋을 지원
}
```
- stride를 부호 있는 타입(`isize`/`i32`)으로 끝까지 유지하고, `usize`로의 캐스팅은 값이 항상 음이 아님을 검증한 이후로 미룬다
- 포인터 이동에는 `add`(음이 아닌 오프셋 전용)가 아니라 `offset`(부호 있는 오프셋 지원)을 사용한다
- negative stride 케이스에 대한 명시적 단위 테스트(가짜 buffer로 offset 계산 검증)를 작성

**탐지 방법**:
- 구조적 검사: `stride`류 필드를 `as usize`로 캐스팅하는 모든 지점을 grep해 부호 손실 여부 검토
- clippy `cast_sign_loss` lint
- Miri/ASan으로 OOB 접근을 감지하는 fuzz 테스트(음수 stride를 흉내낸 mock buffer 포함)

**예외**:
- 사용하는 C 라이브러리가 ABI 문서에서 "stride는 항상 양수"임을 명시적으로 보장하는 경우, 캐스팅 지점에 `debug_assert!(stride > 0)`을 남기고 넘어갈 수 있다

**Bitvue 판정**: Suspected — vvdec 쪽 stride는 `c_uint`(vvdec.rs:79)라 구조적으로 무관하지만, `ffmpeg.rs`는 `frame.stride(N) as usize`를 부호 체크 없이 캐스팅하는 지점이 여럿(ffmpeg.rs:209,219,229,287,291,295). `ffmpeg-next` 6.1.1 소스가 로컬 캐시에 없어 `stride()`가 실제로 음수를 반환할 수 있는 경로(예: 특정 필터 체인 출력)가 있는지 직접 확인하지 못함 — 일반 디코딩 출력에서는 발생 가능성이 낮지만 방어 코드는 없음

---

### FFI-007: size_t/int/usize 변환 손실
**분류**: FFI · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
extern "C" {
    fn vmaf_read_pictures_from_iter(ctx: *mut VmafContext, count: c_int) -> c_int;
}

fn feed_frames(ctx: *mut VmafContext, frame_count: usize) {
    unsafe {
        // usize(64bit) -> c_int(32bit) 암묵적 truncation, 8K/장시간 스트림에서 오버플로 가능
        vmaf_read_pictures_from_iter(ctx, frame_count as c_int);
    }
}
```

**문제**:
- `as` 캐스팅은 오버플로를 검사하지 않는 truncation이므로, 프레임 수·버퍼 크기·offset 같은 값이 `i32`/`c_int` 범위(약 21억)를 넘으면 조용히 음수나 엉뚱한 값으로 wrap됨
- 특히 8K 해상도 프레임의 바이트 크기(`width * height * bytes_per_pixel * planes`)는 `u32`/`c_int` 범위에 근접하거나 넘어설 수 있어, "작은 해상도에서는 통과하고 큰 해상도에서만 터지는" 버그가 된다
- C 쪽 함수가 `size_t`(부호 없음)를 기대하는데 Rust에서 `i32`/`isize`로 잘못 매핑하면 부호 확장/절단으로 큰 값이 음수처럼 해석되어 C 쪽에서 malloc 크기 계산 등에 사용될 경우 힙 오버플로로 이어질 수 있음

**발생 조건**:
- bindgen이 생성한 시그니처와 실제 손으로 작성한 wrapper 시그니처가 어긋나 있고, 그 경계에서 `as` 캐스팅을 습관적으로 사용할 때
- 프레임 수·바이트 길이 등을 다루는 값을 테스트에서는 작은 샘플(수십 프레임, SD 해상도)로만 검증해 큰 값 경로를 타지 않을 때

**권장**:
```rust
fn feed_frames(ctx: *mut VmafContext, frame_count: usize) -> Result<(), FfiError> {
    let count: c_int = frame_count
        .try_into()
        .map_err(|_| FfiError::CountOverflow(frame_count))?;
    unsafe { vmaf_read_pictures_from_iter(ctx, count) };
    Ok(())
}
```
- 경계를 넘는 모든 정수 변환은 `as` 대신 `try_into()`/`TryFrom`을 사용해 실패를 명시적으로 처리한다
- C 헤더의 실제 타입(`size_t` vs `int` vs `unsigned int`)을 bindgen 출력에서 직접 확인하고, 손으로 다시 선언하지 않는다
- 8K/대용량 스트림을 포함한 경계값 테스트(예: 프레임 크기가 `i32::MAX`에 근접하는 케이스)를 CI에 추가

**탐지 방법**:
- clippy `cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap` lint를 FFI 모듈에서 강제
- 구조적 검사: FFI 함수 호출 인자에 있는 `as c_int`/`as u32`/`as usize` 캐스팅을 모두 grep해 `try_into` 대체 가능 여부 검토
- Runtime: 대용량(8K, 장시간) 입력으로 통합 테스트, 실패 시 조용한 wrap이 아니라 명시적 에러가 나는지 확인

**예외**:
- 값의 범위가 타입 정의상 항상 좁음이 보장되는 경우(예: NAL 헤더의 2비트 필드를 담는 값)는 `as`를 써도 안전하지만, 이 경우도 리뷰어가 판단할 수 있도록 근처에 범위 주석을 남긴다

**Bitvue 판정**: Suspected — 위험한 큰 값(payload 크기)에는 이미 `try_from`이 적용(vvdec.rs:592 `i32::try_from(data.len())`)되어 있지만, 검증 후 동일 값을 재차 `as i32`로 캐스팅하는 지점(vvdec.rs:612)과 프레임 인덱스를 무검증 `as u32`로 캐스팅하는 지점(bitvue-metrics/src/vmaf.rs:199,261)이 남아 있음. 실질 위험은 낮음(수십억 단위가 되어야 트리거)이나 패턴 자체는 잔존

---

### FFI-008: C enum 값을 Rust enum으로 unchecked transmute
**분류**: FFI · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
#[repr(i32)]
enum PixelLayout {
    I400 = 0,
    I420 = 1,
    I422 = 2,
    I444 = 3,
}

fn layout_of(pic: &Dav1dPicture) -> PixelLayout {
    unsafe { mem::transmute(pic.p.layout) } // C 라이브러리가 새 값을 추가하면 즉시 UB
}
```

**문제**:
- Rust의 `enum`은 명시된 variant 값만 유효한 것으로 컴파일러가 가정하며, 그 외 값을 가진 enum은 그 자체로 UB(참조/매칭하는 순간이 아니라 그런 값이 "존재하는" 시점부터)
- C 라이브러리는 버전업 시 enum에 새 값을 추가하는 일이 흔하고(dav1d의 pixel layout, libvmaf의 log level, FFmpeg의 pixel format 등), 정적 링크 버전과 헤더 버전이 어긋나면 존재하지 않는 값이 들어올 수 있음
- `match`문이 `PixelLayout::I444`까지만 처리하도록 작성되어 있으면, 새 variant가 하드웨어에서 조용히 잘못된 분기로 빠지거나 컴파일러가 "도달 불가능하다고 가정한 분기"를 최적화로 제거해버려 예측 불가능한 동작이 나온다

**발생 조건**:
- C enum을 Rust enum으로 1:1 매핑하고 `transmute`나 `mem::transmute_copy`로 변환 비용을 아끼려 할 때
- 라이브러리를 헤더 버전 A로 빌드했지만 런타임에 동적 링크된 라이브러리가 버전 B(enum 값 추가됨)인 상황(FFI-021 ABI skew와 결합 가능)

**권장**:
```rust
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PixelLayout {
    I400 = 0,
    I420 = 1,
    I422 = 2,
    I444 = 3,
}

impl TryFrom<i32> for PixelLayout {
    type Error = UnknownPixelLayout;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::I400),
            1 => Ok(Self::I420),
            2 => Ok(Self::I422),
            3 => Ok(Self::I444),
            other => Err(UnknownPixelLayout(other)),
        }
    }
}

fn layout_of(pic: &Dav1dPicture) -> Result<PixelLayout, UnknownPixelLayout> {
    PixelLayout::try_from(pic.p.layout)
}
```
- C enum과의 매핑은 항상 명시적 `match`/`TryFrom`으로 하고, 알 수 없는 값은 명시적 에러(또는 `Unknown(i32)` catch-all variant)로 처리한다
- `#[non_exhaustive]`를 자체 Rust enum에 붙여 향후 값 추가에 대비한 강제 매칭을 유도한다
- 라이브러리 버전업 시 새 enum 값이 추가되었는지 CHANGELOG를 확인하는 절차를 의존성 업데이트 체크리스트에 포함

**탐지 방법**:
- 구조적 검사: `mem::transmute`가 C enum 타입(bindgen이 생성한 정수 alias)과 Rust enum 사이에서 쓰이는 지점을 grep
- clippy에는 이를 직접 잡는 lint가 약하므로, 커스텀 CI 스크립트로 `mem::transmute::<.*Dav1d.*, .*>` 패턴 검색
- Miri로 알려지지 않은 enum 값을 주입하는 테스트(가짜 C 함수로 범위 밖 정수를 반환시켜 검증)

**예외**:
- 없음 — C enum을 신뢰할 수 없는 정수로 취급하고 항상 명시적으로 검증하는 것이 이 카테고리에서는 예외 없는 규칙에 가깝다

**Bitvue 판정**: Suspected — 명시적 `mem::transmute` 호출은 없으나, `VvdecColorFormat`/`VvdecFrameType`이 `#[repr(C)]` Rust enum으로 선언되어(vvdec.rs:84-103) vvdec C 라이브러리가 채워주는 `VvdecFrame` 구조체 메모리에서 `TryFrom` 없이 그대로 읽힘(vvdec.rs:410-433, catch-all match arm은 있어 매칭 자체는 안전). 라이브러리가 새 discriminant를 추가하면 이 필드가 존재하는 시점부터 UB가 될 수 있는 구조이나, vvdec feature 자체가 컴파일되지 않는 상태(FFI-011 참고)라 실제로 트리거된 적은 없음

---

### FFI-009: callback panic이 FFI boundary를 넘어감
**분류**: FFI · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
unsafe extern "C" fn vmaf_log_callback(level: c_int, msg: *const c_char) {
    let msg = unsafe { CStr::from_ptr(msg) }.to_str().unwrap(); // 잘못된 UTF-8이면 panic
    let level = LogLevel::try_from(level).expect("unknown log level"); // 알 수 없는 값이면 panic
    forward_to_app_logger(level, msg);
}
```

**문제**:
- Rust 함수가 C 코드에 함수 포인터로 등록되어 콜백으로 호출될 때, 그 안에서 panic이 발생하면 스택 unwinding이 C 프레임을 통과해야 하는데 C에는 Rust의 unwinding 메커니즘을 처리할 방법이 없어 UB(현실적으로는 프로세스 abort 또는 스택 손상)가 된다
- `extern "C" fn`에서의 panic은 Rust 2021 기준으로 `-C panic=unwind`일 때 명시적으로 UB로 문서화되어 있으며, `panic=abort`라 해도 최소한 프로세스 전체가 즉시 죽는다는 가용성 문제는 남는다
- 로그 콜백처럼 "실패해도 무해해 보이는" 위치일수록 `.unwrap()`/`.expect()`를 방치하기 쉽고, 실제로는 예상 못한 인코딩의 문자열이나 새 로그 레벨 값이 들어오는 순간 전체 디코딩 파이프라인이 죽는다

**발생 조건**:
- dav1d의 할당자 콜백(picture allocator), libvmaf의 로그/progress 콜백처럼 Rust 함수를 C 라이브러리에 함수 포인터로 등록하는 모든 지점
- 콜백 내부에서 `.unwrap()`, `.expect()`, 배열 인덱싱(`[i]`), 산술 오버플로(`+`) 등 panic 가능 연산을 아무 방어 없이 사용할 때

**권장**:
```rust
unsafe extern "C" fn vmaf_log_callback(level: c_int, msg: *const c_char) {
    let result = std::panic::catch_unwind(|| {
        let msg = unsafe { CStr::from_ptr(msg) };
        let msg = msg.to_string_lossy(); // panic 대신 손실 변환
        let level = LogLevel::try_from(level).unwrap_or(LogLevel::Unknown);
        forward_to_app_logger(level, &msg);
    });
    if result.is_err() {
        // 여기서도 절대 panic하지 않는 로깅만 수행
        eprintln!("panic caught inside vmaf_log_callback, suppressed at FFI boundary");
    }
}
```
- extern "C"로 등록되는 모든 Rust 함수의 본문 전체를 `catch_unwind`로 감싸는 것을 규칙화한다(콜백 등록 지점 자체를 매크로/헬퍼로 통일하면 누락을 방지하기 쉽다)
- `catch_unwind` 내부 로직 자체는 panic-free를 지향(`.unwrap()` 대신 fallback 값)해 이중 안전망을 둔다
- 크레이트 전체를 `panic = "abort"`로 빌드하는 선택지도 검토(unwind 자체를 없애 UB 가능성을 원천 차단하지만, 다른 부분에서 정상적인 unwind 기반 정리 로직을 쓸 수 없게 되는 트레이드오프가 있음)

**탐지 방법**:
- 구조적 검사: `extern "C" fn` 시그니처를 가진 함수 전체를 grep하고, 함수 본문이 `catch_unwind`로 감싸져 있는지 확인
- clippy `panic_in_result_fn`은 직접 맞지 않지만, 자체 lint(예: `#![deny(clippy::unwrap_used)]`를 FFI 콜백 모듈에 한정 적용)로 방어
- Runtime: 콜백에 의도적으로 잘못된 입력(비-UTF8 문자열, 범위 밖 enum 값)을 주입해 프로세스가 살아남는지 검증하는 fuzz/통합 테스트

**예외**:
- 콜백이 `panic = "abort"` 빌드 설정이고, "panic 시 프로세스 전체가 즉시 죽어도 무방하다(오히려 원한다)"는 명시적 설계 결정이 있는 경우는 `catch_unwind` 생략이 정당화될 수 있으나, 이 경우도 왜 그런지 주석으로 남긴다

**Bitvue 판정**: N/A — 저장소 전체에 `extern "C" fn` 콜백을 C 라이브러리에 함수 포인터로 등록하는 코드가 없음(grep 결과 0건); dav1d/libvmaf-rs/ffmpeg-next 모두 콜백 기반 API가 아닌 polling 스타일(`get_picture`/`receive_frame`/`read_pictures`)로만 사용됨

---

### FFI-010: C error code를 단일 문자열로 평탄화
**분류**: FFI · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn decode(ctx: *mut Dav1dContext, data: &[u8]) -> Result<(), String> {
    let ret = unsafe { dav1d_send_data(ctx, /* ... */) };
    if ret < 0 {
        return Err("decode failed".to_string()); // EAGAIN도, 손상 비트스트림도, OOM도 동일 메시지
    }
    Ok(())
}
```

**문제**:
- dav1d/FFmpeg 계열은 POSIX errno 스타일의 음수 정수 에러코드(`-EAGAIN`, `-EINVAL`, `-ENOMEM` 등)를 반환하며 각각 의미가 다른데, 이를 전부 동일한 문자열로 뭉개면 호출부가 "재시도해야 하는 상황(EAGAIN)"과 "치명적 손상(EINVAL)"과 "리소스 부족(ENOMEM)"을 구분하지 못한다
- 재시도 가능한 상황까지 치명적 에러로 취급해 정상적인 스트리밍 디코딩 흐름(버퍼링 중 EAGAIN)이 매번 에러 로그/UI 알림을 띄우는 오탐이 발생하거나, 반대로 정말 치명적인 에러도 "재시도하면 되겠지"로 넘어가 무한 루프에 빠질 수 있음
- 상위 계층(UI, 텔레메트리)에서 에러 유형별 분기(예: 재생 중단 vs 경고 표시 vs 자동 복구 시도)를 구현할 수 없게 됨

**발생 조건**:
- 여러 C 함수 호출을 감싸는 wrapper 계층에서 "일단 동작하게" 만들 목적으로 모든 실패를 `String`이나 `anyhow::Error`의 밋밋한 메시지로 뭉뚱그릴 때
- 에러 처리를 나중에 정교화하겠다고 미루고 방치할 때(TODO가 남은 채 방치되는 전형적 패턴)

**권장**:
```rust
#[derive(Debug, thiserror::Error)]
enum DecodeError {
    #[error("decoder needs more input before producing output")]
    NeedMoreData,
    #[error("invalid or corrupt bitstream data")]
    InvalidData,
    #[error("decoder ran out of memory")]
    OutOfMemory,
    #[error("native decoder error (code {0})")]
    Native(i32),
}

fn map_dav1d_error(ret: c_int) -> DecodeError {
    match -ret {
        libc::EAGAIN => DecodeError::NeedMoreData,
        libc::EINVAL => DecodeError::InvalidData,
        libc::ENOMEM => DecodeError::OutOfMemory,
        other => DecodeError::Native(other),
    }
}
```
- C 에러코드를 상위 로직이 분기할 수 있는 구조화된 enum(`thiserror` 등)으로 매핑하고, 원본 코드도 `Native(i32)` variant에 보존한다
- 재시도 가능(EAGAIN류)과 치명적(EINVAL/ENOMEM류) 에러를 타입 수준에서 구분해 호출부가 실수로 섞어 처리하기 어렵게 만든다

**탐지 방법**:
- 구조적 검사: FFI wrapper 함수의 에러 반환 타입이 `String`/`&str`인 곳을 grep해 구조화 에러 타입으로 전환 대상 목록화
- 코드 리뷰: "이 에러를 받은 호출부가 재시도해야 하는지 즉시 판단할 수 있는가"를 체크리스트 항목으로 추가

**예외**:
- 최종 사용자에게 보여줄 로그 메시지 문자열 자체는 평탄화되어도 무방하지만, 그 문자열은 구조화된 에러 값으로부터 `Display`로 파생되어야 하며 판단 로직이 문자열 매칭에 의존해서는 안 된다

**Bitvue 판정**: Confirmed — `crates/bitvue-decode/src/decoder.rs:341-353`에서 dav1d crate가 제공하는 구조화된 `Error::is_again()`(`~/.cargo/registry/.../dav1d-0.10.4/src/lib.rs:55-56`에서 확인)을 쓰지 않고 `err_str.contains("EAGAIN") || err_str.contains("Try again")`로 `Display` 문자열을 파싱해 재시도 여부를 판정 — 정확히 이 항목이 경고하는 "판단 로직이 문자열 매칭에 의존" 사례. 부차적으로 vvdec.rs:739-746도 `VVDEC_TRY_AGAIN`과 `VVDEC_EOF`를 동일한 `DecodeError::NoFrame`으로 뭉개 "재시도 필요"와 "정상 종료"를 구분하지 못함

---

### FFI-011: opaque handle을 Send/Sync로 잘못 선언
**분류**: FFI · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
struct DecoderHandle(*mut Dav1dContext);

// 컴파일 에러를 없애기 위해 아무 검증 없이 선언
unsafe impl Send for DecoderHandle {}
unsafe impl Sync for DecoderHandle {}

// 이제 여러 tokio worker 스레드가 동시에 같은 컨텍스트를 호출할 수 있게 됨
```

**문제**:
- `unsafe impl Send`/`Sync`는 "이 타입을 다른 스레드로 보내거나(Send) 여러 스레드에서 동시 참조해도(Sync) 안전하다"는 컴파일러에 대한 개발자의 약속이지, 실제 C 라이브러리의 스레드 안전성을 검증해주지 않는다
- 많은 디코더/컨텍스트 객체(dav1d의 `Dav1dContext`, libvmaf의 `VmafContext`)는 "동일 컨텍스트에 대한 동시 호출은 안전하지 않다"는 전제로 설계되어 있으며, 내부적으로 락 없는 상태(디코딩 파이프라인 상태, 참조 프레임 버퍼)를 갖는 경우가 흔함
- `Send`만 필요한데(스레드 간 소유권 이전) `Sync`까지 선언하면(여러 스레드에서 동시 `&` 접근 허용) 실제로는 없는 안전성을 코드 상에서 약속하게 되어, 동시 호출 시 내부 상태 레이스로 인한 크래시나 데이터 손상이 발생 — 이는 Rust의 타입 시스템으로 검출되지 않는 순수 UB이므로 재현이 매우 어렵다

**발생 조건**:
- opaque 포인터를 감싼 구조체를 `Arc<Mutex<...>>` 없이 여러 스레드/async task에서 공유하려다 컴파일 에러(`*mut T`는 기본적으로 `!Send`/`!Sync`)를 만나고, 원인을 분석하지 않고 `unsafe impl`로 우회할 때
- 라이브러리 문서에 스레드 안전성이 명시되어 있지 않거나(흔함), 있어도 확인하지 않고 넘어갈 때

**권장**:
```rust
struct DecoderHandle(*mut Dav1dContext);

// dav1d 문서: 하나의 Dav1dContext는 단일 논리적 호출자만 사용해야 하며
// 내부적으로 스레드 풀을 관리하지만 컨텍스트 자체에 대한 외부 동시 호출은 지원하지 않음.
// 따라서 Send는 허용(소유권 이전은 안전)하되 Sync는 선언하지 않는다.
unsafe impl Send for DecoderHandle {}
// Sync는 의도적으로 구현하지 않음 — 동시 접근이 필요하면 Mutex<DecoderHandle>로 감쌀 것
```
- `Send`/`Sync` 각각을 라이브러리 문서(또는 소스 코드의 스레드 안전성 주석)를 근거로 개별적으로 판단하고, 근거를 코드 주석에 남긴다
- 실제 동시 접근이 필요하면 `Mutex<DecoderHandle>`로 감싸 Rust의 안전한 동시성 보장을 그대로 활용한다(자체적으로 `Sync`를 선언해 락 없는 동시 접근을 허용하지 않는다)
- 가능하면 라이브러리 소스(C 코드)에서 전역/컨텍스트 상태에 대한 동기화 여부를 직접 확인하거나, 업스트림 이슈 트래커에서 스레드 안전성 논의를 찾는다

**탐지 방법**:
- 구조적 검사: `unsafe impl Send`/`unsafe impl Sync` 전체를 grep해 각각에 대해 "왜 안전한가"를 설명하는 주석이 붙어 있는지 감사
- Runtime: ThreadSanitizer(TSan)로 빌드해 동일 핸들에 대한 동시 호출 시나리오를 통합 테스트로 실행, 데이터 레이스 탐지
- 코드 리뷰: 새로 추가되는 `unsafe impl Send/Sync`는 반드시 별도 승인 라인을 요구하는 리뷰 규칙 도입

**예외**:
- 라이브러리가 명시적으로 "컨텍스트는 내부적으로 완전히 스레드 안전하다"고 문서화하고, 그 근거(내부 락, 스레드-로컬 상태 없음)를 소스 레벨에서 확인한 경우에만 `Sync`도 정당화된다

**Bitvue 판정**: Confirmed — `Decoder` 트레이트가 `Send` 슈퍼트레이트를 요구(`crates/bitvue-decode/src/traits.rs:127` `pub trait Decoder: Send`)하지만, `VvcDecoder`는 raw pointer 필드(`Mutex<*mut ffi::VvdecDecoder>`, `Mutex<*mut ffi::VvdecAccessUnit>`) 때문에 자동으로 `!Send`이고, 코드 주석은 "vvdec may have race conditions" 때문에 `Send` impl을 의도적으로 생략했다고 명시(vvdec.rs:258-259) — 그 결과 `impl Decoder for VvcDecoder`가 컴파일되지 않음. 직접 `cargo check -p bitvue-decode --features vvdec` 실행 결과 `*mut c_void`/`*mut VvdecAccessUnit`/`*mut VvdecFrame`에 대해 "cannot be sent between threads safely" E0277가 vvdec.rs:566,638,673,760에서 발생함을 확인(총 12개 컴파일 에러, DecodedFrame 필드 타입 불일치 E0308 3건 포함) — 이 파일은 vvdec feature로 빌드된 적이 사실상 없는 상태(CI에도 vvdec 관련 언급 없음)

---

### FFI-012: native object clone처럼 보이는 shallow copy
**분류**: FFI · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
#[derive(Clone)] // derive만으로 "값 복사"가 안전하다고 착각하게 만듦
struct FramePicture {
    inner: Dav1dPicture, // 내부에 refcount 포인터(ref: *mut Dav1dRef)를 담고 있음
}

fn duplicate_for_analysis(frame: &FramePicture) -> FramePicture {
    frame.clone() // Dav1dPicture 구조체 필드를 바이트 단위로 복사할 뿐, dav1d ref count는 증가하지 않음
}
```

**문제**:
- `#[derive(Clone)]`은 필드를 재귀적으로 clone하지만, `Dav1dPicture` 같은 C struct는 `#[repr(C)]`로 bindgen이 생성한 POD(plain-old-data) 타입이라 `Clone`이 사실상 `Copy`와 동일한 "얕은 바이트 복사"가 된다 — 내부의 `*mut Dav1dRef` 같은 refcount 포인터는 값만 복사되고 dav1d 라이브러리 쪽 refcount는 증가하지 않는다
- 두 개의 `FramePicture` 인스턴스가 동일한 native 버퍼를 "각자 소유"한다고 착각한 채 각각 `Drop`(또는 수동 `dav1d_picture_unref`)을 호출하면, 동일 refcount 객체에 대해 unref가 두 번 일어나 두 번째 unref 시점에 double free
- "clone"이라는 이름 자체가 강한 신호(값 의미론)를 주기 때문에, 이 shallow copy가 위험하다는 사실이 호출부 코드만 봐서는 전혀 드러나지 않는다 — API 설계 실수가 사용자 실수를 유도하는 전형적 사례

**발생 조건**:
- bindgen이 생성한 `#[repr(C)]` struct에 편의를 위해 그대로 `#[derive(Clone, Copy)]`를 붙이거나, 붙이지 않았더라도 수동으로 필드별 복사를 구현할 때
- refcount 기반 리소스(대부분의 디코더 프레임, GPU 버퍼 핸들)를 값처럼 다루고 싶어서 안전 래퍼 설계를 생략할 때

**권장**:
```rust
struct FramePicture {
    inner: Dav1dPicture,
}

impl Clone for FramePicture {
    fn clone(&self) -> Self {
        let mut inner = self.inner;
        unsafe { dav1d_picture_ref(&mut inner, &self.inner as *const _ as *mut _) };
        // native refcount를 실제로 증가시킨 뒤에만 두 번째 값을 만든다
        FramePicture { inner }
    }
}

impl Drop for FramePicture {
    fn drop(&mut self) {
        unsafe { dav1d_picture_unref(&mut self.inner) };
    }
}
```
- refcount가 있는 native 객체를 감싸는 타입에는 `#[derive(Clone)]`을 쓰지 않고, native ref API(`dav1d_picture_ref` 등)를 실제로 호출하는 수동 `Clone` 구현을 작성한다
- native ref API가 없는 리소스(진짜 단일 소유만 가능한 경우)라면 애초에 `Clone`을 구현하지 않아 컴파일 타임에 복제 시도를 막는다
- bindgen 원시 타입에는 `Clone`/`Copy`를 파생하지 않고, 안전 래퍼 계층에서만 의미론에 맞게 구현한다(FFI-019 참고)

**탐지 방법**:
- 구조적 검사: bindgen 생성 타입 또는 raw pointer를 필드로 가진 struct에 붙은 `#[derive(Clone)]`/`#[derive(Copy)]`를 모두 grep해 개별 검토
- Miri/ASan으로 clone 후 양쪽에서 drop하는 시나리오를 테스트에 명시적으로 포함해 double free 탐지
- 코드 리뷰: "이 타입을 clone하면 native 쪽 refcount도 증가하는가"를 새 wrapper 타입 도입 시 체크리스트에 포함

**예외**:
- struct가 native 리소스에 대한 포인터를 전혀 담고 있지 않고 순수 값(치수, 플래그 등)만 담는다면 derive Clone/Copy는 안전하다

**Bitvue 판정**: N/A — `DecodedFrame`(decoder.rs:32-61)은 `#[derive(Clone)]`이지만 plane 데이터가 `Arc<[u8]>`(Rust 자체 refcount)일 뿐 C library refcount 포인터를 필드로 갖지 않음; vvdec의 `VvdecFrame`/`VvdecPlane`류 raw FFI 구조체에는 `Clone`/`Copy`가 derive되어 있지 않음

---

### FFI-013: CString 내부 NUL 처리 누락
**분류**: FFI · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn open_log_file(path: &str) -> *mut Dav1dContext {
    let c_path = CString::new(path).unwrap(); // 경로에 NUL 바이트가 있으면 panic
    unsafe { dav1d_open_with_log_path(c_path.as_ptr()) }
}
```

**문제**:
- `CString::new`은 입력에 내부 NUL 바이트(`\0`)가 있으면 `Err(NulError)`를 반환하는데, `.unwrap()`으로 처리하면 그 즉시 panic — 파일 경로, 사용자 입력 메타데이터, 자막/태그 문자열 등 외부에서 유래한 문자열은 이 가능성을 배제할 수 없다
- 이 panic이 FFI 콜백 내부에서 발생하면 FFI-009와 결합해 unwind-across-FFI UB로 이어질 수 있음(예: 로그 콜백 안에서 파일 경로를 CString화하는 경우)
- NUL 바이트를 조용히 제거/치환하는 것도 위험한 대안이다 — 파일 경로의 의미가 바뀌어 의도치 않은 파일을 열거나 덮어쓸 수 있음

**발생 조건**:
- 사용자가 지정 가능한 파일 경로, 스트림 메타데이터(타이틀, 태그), 커맨드라인 인자 등 신뢰할 수 없는 출처의 문자열을 C API에 넘길 때
- "실무에서는 NUL이 포함된 경로가 거의 없다"는 가정으로 에러 처리를 생략할 때(실제로는 손상된 비트스트림 메타데이터에서 NUL이 나타날 수 있음)

**권장**:
```rust
fn open_log_file(path: &str) -> Result<*mut Dav1dContext, OpenError> {
    let c_path = CString::new(path).map_err(|_| OpenError::PathContainsNul)?;
    Ok(unsafe { dav1d_open_with_log_path(c_path.as_ptr()) })
}
```
- `CString::new`의 `Result`를 항상 명시적으로 처리하고, 실패 시 사용자에게 이해 가능한 에러(경로에 NUL 문자 포함 등)로 변환한다
- FFI 콜백 내부에서 문자열을 CString화해야 한다면 FFI-009의 `catch_unwind` 규칙과 결합해 이중으로 방어한다
- 신뢰할 수 없는 문자열이 C API 경계를 넘는 지점을 인벤토리화해 일괄 검토한다

**탐지 방법**:
- clippy에는 직접적인 lint가 없으므로, 구조적 검사로 `CString::new(...).unwrap()` / `.expect(...)` 패턴을 grep
- 코드 리뷰: 새 `CString::new` 호출 추가 시 에러 경로 처리 여부를 체크리스트로 확인
- fuzzing: 경로/메타데이터 문자열 생성기에 NUL 바이트를 포함한 케이스를 의도적으로 포함

**예외**:
- 문자열이 컴파일 타임 상수이거나 프로그램 내부에서 완전히 통제되는 값(NUL 포함 가능성이 구조적으로 없음)이라면 `.expect("no interior nul in constant")`로 의도를 명시하고 넘어갈 수 있다

**Bitvue 판정**: N/A — 저장소 전체에 `CString::new` 호출이 없음(grep 결과 0건); vvdec/dav1d/ffmpeg-next/libvmaf-rs 어느 경로도 문자열을 C API에 CString으로 전달하지 않음(파일 경로 등은 모두 Rust `Path`/`&str` 기반 API로 처리)

---

### FFI-014: ownership transfer 여부 불명확
**분류**: FFI · **심각도**: Critical · **탐지**: Semantic

**나쁜 예**:
```rust
extern "C" {
    // 이 함수가 picture의 소유권을 가져가는지(내부에서 free), 아니면 빌려서 쓰기만 하는지
    // 함수 이름과 시그니처만으로는 알 수 없음
    fn vmaf_feed_picture(ctx: *mut VmafContext, pic: *mut VmafPicture) -> c_int;
}

fn submit(ctx: *mut VmafContext, mut pic: VmafPicture) {
    unsafe { vmaf_feed_picture(ctx, &mut pic) };
    // 호출자는 여전히 pic을 소유했다고 믿고 이후 재사용하거나 해제 시도
    unsafe { vmaf_picture_unref(&mut pic) }; // 이미 라이브러리가 내부에서 가져갔다면 double free,
                                              // 반대로 라이브러리가 안 가져갔다면 이 줄이 없으면 leak
}
```

**문제**:
- C API의 "포인터를 넘긴다"는 동작은 최소 세 가지 의미(빌림/borrow, 소유권 이전/move, 소유권 이전 후 콜백으로 나중에 돌려줌)를 가질 수 있는데, 시그니처만 봐서는 구분할 수 없고 오직 문서 또는 소스 코드 확인으로만 알 수 있다
- 소유권 가정이 틀리면 두 가지 상반된 버그가 나온다: 라이브러리가 가져갔는데 호출자도 해제하면 double free, 호출자가 계속 소유한다고 오해해 해제를 누락하면 leak
- 이 문제는 API마다, 심지어 같은 라이브러리의 함수마다 규칙이 다를 수 있어("이 함수는 가져가지만 저 함수는 안 가져간다") 일관된 가정을 세울 수 없고 개별 확인이 필요하다

**발생 조건**:
- 새로운 C API를 처음 바인딩할 때 공식 문서가 불충분하거나 없어서 함수 이름/파라미터 이름(`take_ownership` 같은 힌트 부재)만으로 추측할 때
- 기존 코드를 복사-붙여넣기하며 다른 함수에 적용했는데 그 함수의 소유권 규칙이 실제로는 다를 때

**권장**:
```rust
// vmaf.h 주석 확인 결과: vmaf_feed_picture는 성공 시 pic의 소유권을 라이브러리로 이전하며,
// 라이브러리가 처리 완료 후 내부적으로 unref한다. 실패 시(음수 반환) 호출자가 여전히 소유한다.
fn submit(ctx: *mut VmafContext, pic: VmafPicture) -> Result<(), FeedError> {
    let mut pic = ManuallyDrop::new(pic); // 성공 경로에서는 Rust Drop이 개입하지 않도록
    let ret = unsafe { vmaf_feed_picture(ctx, &mut pic.inner) };
    if ret < 0 {
        // 실패: 소유권이 반환되었으므로 여기서만 명시적으로 drop
        unsafe { ManuallyDrop::drop(&mut pic) };
        return Err(FeedError::Native(ret));
    }
    Ok(()) // 성공: 소유권은 이미 라이브러리로 이전됨, Rust 쪽에서 추가 해제하지 않음
}
```
- 각 C 함수의 소유권 규칙을 바인딩 코드 주석에 "문서 근거 + 실제 소스 확인 결과"로 명시한다(추측이 아니라 확인된 사실로)
- 소유권이 이전되는 값은 `ManuallyDrop`으로 감싸 Rust의 자동 drop이 개입하지 않게 하고, 소유권이 실제로 돌아오는 경로(에러 시 등)에서만 명시적으로 drop한다
- 불명확한 API는 업스트림 소스 코드(C 구현)를 직접 읽어 확인하거나, 메인테이너에게 문의해 문서에 없는 계약을 확정한 뒤 진행한다

**탐지 방법**:
- 구조적 검사: 포인터를 인자로 받는 모든 extern 함수 호출부에 "소유권 계약" 주석이 있는지 감사(없으면 리뷰에서 반려)
- Runtime: Miri/ASan으로 성공/실패 양쪽 경로 모두를 커버하는 테스트(특히 실패 경로가 double free/leak를 유발하는지)
- 코드 리뷰: 새 C API를 처음 바인딩하는 PR에는 "소유권 계약을 어디서 확인했는가"를 필수 항목으로 요구

**예외**:
- 없음 — 소유권 계약이 불명확한 채로 코드를 작성하는 것 자체가 이 카테고리의 핵심 위험이므로, 확인 전에는 최소한 `ManuallyDrop` + 명시적 TODO로 리스크를 표시해 둔다

**Bitvue 판정**: N/A — vvdec 쪽은 소유권 규칙이 명확히 지켜짐: `convert_frame` 성공/실패 무관 항상 `vvdec_frame_unref` 호출(vvdec.rs:708-722, 주석으로 근거 명시); dav1d/libvmaf-rs는 wrapper crate가 소유권을 안전하게 캡슐화해 Bitvue 코드에서 소유권 판단이 필요한 raw pointer 인자가 없음

---

### FFI-015: free 함수가 다른 allocator를 사용
**분류**: FFI · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
fn make_extra_data(codec_params: &[u8]) -> *mut u8 {
    let boxed: Box<[u8]> = codec_params.to_vec().into_boxed_slice();
    Box::into_raw(boxed) as *mut u8 // Rust global allocator(jemalloc/system)로 할당
}

extern "C" {
    // C 라이브러리 내부는 malloc/free로 관리되는 것을 전제
    fn dav1d_data_wrap(data: *const Dav1dData, buf: *const u8, sz: usize,
                        free_callback: extern "C" fn(*const u8, *mut c_void), cookie: *mut c_void) -> c_int;
}
```

**문제**:
- Rust의 `Box`/`Vec`가 사용하는 전역 allocator(기본은 시스템 allocator지만 jemalloc 등으로 교체될 수 있음)와 C 라이브러리가 내부적으로 기대하는 allocator(보통 libc `malloc`/`free`)가 다를 수 있는데, 한쪽에서 할당한 메모리를 다른 쪽의 free 함수로 해제하면 힙 메타데이터 불일치로 인한 힙 손상(heap corruption)이 발생 — 이는 즉시 크래시하지 않고 나중에 무관한 코드 위치에서 터지는 가장 디버깅하기 어려운 UB 유형 중 하나
- 위 예시처럼 free 콜백을 등록하지 않거나 잘못 등록하면(Rust `Box`로 만든 메모리를 C의 `free()`로 해제하도록 콜백을 연결) 이 문제가 그대로 재현됨
- 반대 방향(C가 malloc한 메모리를 Rust `Box::from_raw`로 감싸 Rust의 `Drop`이 Rust allocator의 `dealloc`을 호출하게 만드는 경우)도 동일한 문제를 일으킨다

**발생 조건**:
- Rust에서 할당한 버퍼를 C API에 넘기면서 "이 메모리를 다 쓰면 어떻게 해제할지"를 명시하는 콜백/함수 포인터 파라미터가 있는 API를 사용할 때
- C 라이브러리가 반환한 포인터(`malloc`으로 할당됨)를 Rust 쪽에서 `Box::from_raw` 등으로 감싸 Rust의 자동 drop에 맡길 때
- 프로젝트가 커스텀 global allocator(jemalloc, mimalloc 등)를 설정한 상태에서 C 라이브러리와 메모리를 주고받을 때(가장 흔한 실제 촉발 조건)

**권장**:
```rust
// Rust가 할당한 메모리를 C에 넘길 때는, "해제도 Rust가 한다"는 콜백을 명시적으로 등록한다
extern "C" fn free_rust_boxed_slice(_data: *const u8, cookie: *mut c_void) {
    unsafe {
        // cookie에 Box::into_raw로 만든 원본 fat pointer 정보를 안전하게 복원
        let boxed = Box::from_raw(cookie as *mut Vec<u8>);
        drop(boxed); // Rust allocator로 해제 — Rust가 할당한 메모리는 Rust가 해제
    }
}

fn make_extra_data(codec_params: &[u8]) -> (*const u8, *mut c_void) {
    let boxed = Box::new(codec_params.to_vec());
    let ptr = boxed.as_ptr();
    let cookie = Box::into_raw(boxed) as *mut c_void;
    (ptr, cookie) // free_rust_boxed_slice와 cookie를 함께 dav1d_data_wrap에 등록
}
```
- "이 메모리는 누가 할당했고, 누가 해제하는가"를 allocator 단위로 끝까지 대칭적으로 유지한다: Rust가 할당했으면 Rust가 해제하는 콜백을 등록하고, C가 할당했으면 C의 free 함수(바인딩된 `free` 심볼)로 해제한다
- C 라이브러리가 커스텀 allocator 콜백을 등록할 수 있는 API를 제공한다면(예: dav1d의 picture allocator), 이를 활용해 애초에 allocator를 하나로 통일하는 것도 방법
- 프로젝트에서 커스텀 global allocator를 쓴다면, FFI 경계를 넘는 모든 메모리에 대해 이 문제를 특히 신경 써서 감사한다

**탐지 방법**:
- 구조적 검사: `Box::into_raw`/`Box::from_raw`와 C `free`/커스텀 free 콜백이 짝을 이루는 지점을 모두 찾아 allocator 출처가 일치하는지 확인
- Runtime: ASan(malloc/free mismatch 감지 기능 포함), 또는 valgrind의 `--tool=memcheck`로 "mismatched free() / delete / delete[]" 경고 확인
- 코드 리뷰: 새 FFI 메모리 전달 지점마다 "할당자"와 "해제자"를 한 줄로 명시하는 주석을 요구

**예외**:
- 없음 — allocator 대칭성은 항상 지켜야 하는 불변 조건이다. 다만 플랫폼의 시스템 allocator만 사용하고(커스텀 global allocator 미사용) libc `malloc`/`free`와 실제로 동일 구현을 공유하는 것이 보장되는 특수 환경이라면 실무적 위험은 낮아지지만, 여전히 명시적으로 문서화할 것을 권장한다

**Bitvue 판정**: N/A — 저장소 어디에도 `#[global_allocator]` 커스텀 설정이 없음(grep 결과 0건, 기본 시스템 allocator만 사용); vvdec 버퍼는 항상 vvdec 자체 alloc/free 쌍(`vvdec_accessUnit_alloc_payload`/`vvdec_accessUnit_free_payload`)으로만 관리되고 Rust `Box`로 할당한 메모리를 C free 콜백에 넘기는 코드가 없음

---

### FFI-016: thread affinity가 필요한 객체를 임의 thread에서 호출
**분류**: FFI · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
struct VmafSession {
    ctx: *mut VmafContext,
}

async fn score_frame(session: Arc<Mutex<VmafSession>>, frame: FramePicture) -> f64 {
    // tokio worker 스레드 중 아무 곳에서나 실행될 수 있음; VmafContext가
    // 생성 시점의 스레드에 바인딩된 내부 리소스(스레드 풀, TLS 상태)를 가질 수 있음
    let session = session.lock().unwrap();
    unsafe { vmaf_score(session.ctx, frame.as_ptr()) }
}
```

**문제**:
- 일부 C 라이브러리(특히 내부적으로 스레드 풀이나 GPU 컨텍스트, 코덱 하드웨어 가속 핸들을 갖는 경우)는 "생성한 스레드에서만 호출 가능" 또는 "호출 스레드가 매번 같아야 함" 같은 thread-affinity 제약을 가질 수 있는데, `Mutex`로 감싼다고 해서 이 제약이 자동으로 지켜지지는 않는다 — `Mutex`는 동시 접근을 막을 뿐 "어느 스레드에서 호출되는가"는 보장하지 못한다
- tokio 같은 async 런타임의 worker 스레드는 매 poll마다 다른 OS 스레드에서 실행될 수 있어(`work-stealing` 스케줄러), thread-affinity가 필요한 객체를 async 컨텍스트에서 직접 다루면 이 제약이 조용히 깨진다
- 증상이 "가끔 크래시" 또는 "가끔 잘못된 결과"로 나타나고 스레드 스케줄링에 의존하므로 재현이 매우 어렵고, 로컬 개발 환경(코어 적음, 스케줄링 패턴 다름)에서는 재현되지 않다가 프로덕션(코어 많음)에서만 터지는 경우가 흔하다

**발생 조건**:
- GPU 가속 디코딩/필터 경로, 또는 내부적으로 스레드 로컬 상태를 쓰는 C 라이브러리를 async 런타임 위에서 직접 호출할 때
- 라이브러리 문서에 thread-affinity 요구사항이 명시되어 있지 않아 존재 자체를 모르고 지나칠 때(가장 흔한 촉발 원인)

**권장**:
```rust
struct VmafSession {
    ctx: *mut VmafContext,
}

// thread-affinity가 필요한 세션은 전용 OS 스레드에 고정하고, 그 스레드로만 작업을 보낸다
fn spawn_vmaf_worker() -> (JoinHandle<()>, Sender<VmafJob>) {
    let (tx, rx) = std::sync::mpsc::channel::<VmafJob>();
    let handle = std::thread::spawn(move || {
        let session = VmafSession::new(); // 이 스레드에서 생성 = 이 스레드에 귀속
        for job in rx {
            let result = unsafe { vmaf_score(session.ctx, job.frame.as_ptr()) };
            let _ = job.reply.send(result);
        }
    });
    (handle, tx)
}
```
- thread-affinity가 의심되거나 확인된 객체는 async 런타임의 worker pool에서 직접 다루지 않고, 전용 OS 스레드(생성부터 소멸까지 동일 스레드) + 채널 기반 작업 큐로 격리한다
- `tokio::task::spawn_blocking`도 매 호출마다 다른 스레드를 쓸 수 있으므로 thread-affinity 요구가 있는 객체에는 부적합하다 — 반드시 고정된 단일 스레드를 사용한다
- 라이브러리의 스레드 안전성/친화성 문서를 확인할 수 없다면, 보수적으로 전용 스레드 격리를 기본값으로 채택한다

**탐지 방법**:
- Runtime: ThreadSanitizer(TSan)로 실제 호출 스레드 ID가 세션 수명 동안 바뀌는지 계측하는 통합 테스트 작성(예: 호출 시 `std::thread::current().id()`를 로깅해 일관성 검증)
- 부하 테스트: worker 스레드 수를 늘리고 async 태스크를 다수 동시 실행해 간헐적 크래시가 재현되는지 스트레스 테스트
- 코드 리뷰: async 함수 본문에서 C FFI 호출이 직접 이루어지는 지점을 모두 찾아 해당 라이브러리의 thread-affinity 여부를 개별 확인

**예외**:
- 라이브러리가 명시적으로 "컨텍스트당 임의 스레드에서 호출 가능"이라고 문서화하고, 내부적으로 스레드 풀/TLS를 쓰지 않음이 확인된 경우는 예외로 async 컨텍스트에서 직접 호출해도 안전하다

**Bitvue 판정**: Suspected — `run_decode_with_timeout`(vvdec.rs:285-317)가 decode/flush 호출마다 새 OS 스레드를 `thread::spawn`으로 생성해 그 안에서 raw pointer로 FFI를 호출함(vvdec.rs:673-681). vvdec가 실제 thread-affinity를 요구하는지 업스트림 문서를 직접 확인하지 못했고(소프트웨어 디코더라 가능성은 낮음), 무엇보다 이 파일은 FFI-011의 Send 컴파일 에러 때문에 vvdec feature로 빌드 자체가 되지 않아 런타임에서 검증된 적이 없음

---

### FFI-017: library global initialization을 여러 번 수행
**분류**: FFI · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
fn create_decoder() -> *mut Dav1dContext {
    unsafe {
        av_log_set_level(AV_LOG_INFO); // FFmpeg류 전역 초기화를 디코더 생성마다 반복 호출
    }
    let mut ctx = ptr::null_mut();
    unsafe { dav1d_open(&mut ctx, &settings) };
    ctx
}

// 테스트마다, 요청마다 create_decoder()가 호출됨 → 전역 초기화도 그만큼 반복됨
```

**문제**:
- 일부 C 라이브러리의 전역 초기화 함수는 멱등(idempotent)하지 않다 — 두 번째 호출이 첫 번째 호출로 등록된 전역 상태(로그 핸들러, 콜백 테이블, 레지스트리)를 덮어쓰거나, 내부 카운터를 다시 증가시켜 나중에 "초기화 해제" 시점 계산이 어긋나거나, 최악의 경우 이미 초기화된 자원을 다시 할당해 leak/crash를 유발할 수 있음
- 병렬 테스트 실행(예: `cargo test`의 기본 동작인 스레드별 병렬 테스트) 환경에서 여러 테스트가 동시에 같은 전역 초기화 함수를 호출하면, 초기화 자체가 스레드 안전하지 않은 라이브러리의 경우 레이스 컨디션이 발생
- "매번 호출해도 무해해 보인다"는 직관과 달리, 실제로 문제가 되는 경우는 드물게만 트리거되어(대부분은 우연히 안전) 발견이 늦어지고, 발견되었을 때는 이미 코드베이스 전체에 이 패턴이 퍼져 있는 경우가 많다

**발생 조건**:
- 디코더/컨텍스트 생성 함수 안에 "혹시 몰라서" 전역 초기화 호출을 끼워 넣고, 이 생성 함수가 요청마다·테스트마다 반복 호출될 때
- 전역 초기화가 필요하다는 사실 자체를 모르고 여러 모듈에서 각자 독립적으로 초기화 코드를 추가할 때(중복의 존재를 아무도 인지하지 못함)

**권장**:
```rust
use std::sync::Once;

static INIT: Once = Once::new();

fn ensure_global_init() {
    INIT.call_once(|| {
        unsafe { av_log_set_level(AV_LOG_INFO) };
        // 그 외 프로세스 전체에 한 번만 필요한 초기화
    });
}

fn create_decoder() -> *mut Dav1dContext {
    ensure_global_init(); // 여러 번 호출해도 실제 초기화는 1회만 수행됨
    let mut ctx = ptr::null_mut();
    unsafe { dav1d_open(&mut ctx, &settings) };
    ctx
}
```
- `std::sync::Once`(또는 `OnceLock`)로 전역 초기화를 프로세스 생애 동안 정확히 1회만 실행되도록 강제한다
- 전역 초기화가 필요한 라이브러리 목록과 그 근거를 한 곳(예: `ffi::init` 모듈)에 모아 중복 추가를 방지한다
- 테스트 하네스에서도 동일한 `Once` 기반 초기화 함수를 공유해, 테스트별로 별도 초기화 로직을 만들지 않는다

**탐지 방법**:
- 구조적 검사: 전역 초기화로 알려진 함수명(예: `av_log_set_level`, `*_global_init`, `*_register_all`)의 호출 지점을 grep해 `Once`로 감싸져 있는지 확인
- Runtime: `cargo test -- --test-threads=N`(N>1)로 여러 테스트를 병렬 실행해 초기화 관련 레이스/크래시가 재현되는지 확인
- 코드 리뷰: 디코더/컨텍스트 생성 함수 내부에 전역 초기화 호출이 인라인되어 있으면 `Once` 패턴으로 리팩터링 요청

**예외**:
- 라이브러리가 초기화 함수의 멱등성을 명시적으로 보장하는 경우(문서에 "여러 번 호출해도 안전"이라고 명시) 예외이나, 이 경우도 `Once`를 쓰는 편이 의도를 더 명확히 드러내므로 굳이 반복 호출을 유지할 이유는 적다

**Bitvue 판정**: Suspected — `FfmpegDecoder::new()`가 호출될 때마다 `ffmpeg::init()`을 호출(ffmpeg.rs:70)하고, `reset()`은 `*self = Self::new(...)`로 전체 재생성(ffmpeg.rs:388-392)해 init을 반복 호출함; Bitvue 코드에는 `Once`/`OnceLock` 가드가 없음. `ffmpeg-next` 6.1.1 소스가 로컬 캐시에 없어 crate 내부에 자체 Once 가드가 있는지 직접 확인하지 못함 — 있다면 실무 위험은 낮고, 없다면 병렬 테스트(`cargo test` 기본 동작)에서 레이스 가능

---

### FFI-018: shutdown 순서가 callback보다 먼저 실행
**분류**: FFI · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
struct AppState {
    vmaf_ctx: *mut VmafContext,
    metrics_sink: Box<dyn Fn(f64) + Send>, // vmaf 콜백이 이 클로저를 호출
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe { vmaf_close(self.vmaf_ctx) }; // 컨텍스트를 먼저 닫음
        // metrics_sink(Box)는 AppState 필드 순서대로 그 다음에 drop됨
        // 하지만 vmaf_close가 비동기적으로 아직 진행 중인 콜백을 기다리지 않는다면?
    }
}
```

**문제**:
- 많은 C 라이브러리의 콜백은 등록한 쪽(Rust)이 만든 함수 포인터/클로저를 호출하는데, 그 콜백이 참조하는 Rust 쪽 상태(클로저가 캡처한 데이터, `user_data` 포인터가 가리키는 객체)가 라이브러리의 shutdown/close보다 먼저 해제되면 use-after-free가 된다
- 문제는 "라이브러리를 먼저 닫고 콜백 관련 리소스를 나중에 정리한다"는 순서가 아니라, 그 반대 방향도 위험하다는 데 있다: close 함수가 "진행 중인 비동기 작업/콜백이 모두 끝날 때까지 블로킹한다"는 보장이 없는 라이브러리라면, close가 반환된 직후에도 백그라운드 스레드에서 콜백이 한 번 더 실행될 수 있다
- Rust의 필드 drop 순서(선언 순서, 역순으로 drop)에 의존해 "콜백 관련 리소스가 나중에 정리되니 안전하다"고 가정하는 것은, 그 리소스를 정리하는 시점에 아직 라이브러리 내부에서 콜백이 실행 중일 수 있다는 사실을 놓친 것

**발생 조건**:
- 비동기/스레드 풀 기반으로 동작하는 C 라이브러리(로그 콜백, progress 콜백, 프레임 완료 콜백을 백그라운드 스레드에서 호출하는 구조)를 사용할 때
- 애플리케이션 종료(그리고 테스트 종료) 경로에서 `Drop` 구현 순서에만 의존하고, 라이브러리가 제공하는 "모든 콜백이 끝났음을 보장하는 API"(있다면)를 명시적으로 기다리지 않을 때

**권장**:
```rust
struct AppState {
    vmaf_ctx: *mut VmafContext,
    metrics_sink: Arc<dyn Fn(f64) + Send + Sync>, // Arc로 콜백이 자신의 몫을 들고 있게 함
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            // 라이브러리가 "진행 중 콜백을 모두 flush/join한 뒤에만 반환"을 보장하는
            // API가 있다면 반드시 그것을 사용한다. 없다면 자체적으로 진행 중 작업 카운터를
            // 두고 여기서 그 카운터가 0이 될 때까지 대기한다.
            vmaf_close(self.vmaf_ctx);
        }
        // metrics_sink는 Arc이므로, 만약 close 직후에도 아주 짧게 콜백이 실행 중이었다면
        // 그 콜백이 들고 있던 Arc 클론이 실제 데이터의 마지막 소유자가 되어 안전하게 정리된다
    }
}
```
- close/shutdown 함수가 "모든 진행 중 콜백 완료를 보장한 뒤 반환"하는지 라이브러리 문서에서 반드시 확인하고, 보장하지 않는다면 자체적으로 진행 중 작업 카운터(`Arc<AtomicUsize>` 등)를 두어 0이 될 때까지 명시적으로 대기하는 로직을 shutdown 경로에 추가한다
- 콜백이 캡처하는 리소스는 단순 소유(`Box`)가 아니라 `Arc`로 공유해, "콜백이 아직 실행 중인데 메인 쪽이 먼저 정리했다"는 race가 발생해도 최소한 use-after-free 대신 정상적인 참조 카운팅 정리로 귀결되게 한다
- 종료 경로 전용 통합 테스트(디코딩 진행 중 강제 shutdown을 반복 실행)를 CI에 포함한다

**탐지 방법**:
- Runtime: TSan + ASan을 함께 사용해 shutdown 경로와 콜백 실행 경로 사이의 레이스를 탐지하는 스트레스 테스트(짧은 간격으로 생성→진행 중 shutdown을 반복)
- 구조적 검사: `Drop` 구현에서 close류 함수를 호출하는 곳마다, 그 라이브러리의 close API가 "콜백 완료 보장" 문서를 갖는지 인벤토리화
- 코드 리뷰: 새 콜백 등록 API를 도입할 때 "이 콜백에 대한 shutdown-safety는 어떻게 보장되는가"를 필수 질문으로 추가

**예외**:
- 콜백이 완전히 동기적으로만 호출되고(라이브러리가 별도 스레드/큐를 쓰지 않음이 확인됨) close 함수가 반환되는 순간 콜백 호출 가능성이 완전히 사라짐이 보장된 경우는 Drop 순서만으로 충분할 수 있다

**Bitvue 판정**: N/A — 등록된 콜백이 없으므로(FFI-009 참고) shutdown-vs-콜백 순서 문제 자체가 성립하지 않음; `VvcDecoder`/`FfmpegDecoder`의 `Drop`은 모두 동기적 close/free 호출만 수행(vvdec.rs:855-893)

---

### FFI-019: bindgen 결과를 domain API로 직접 노출
**분류**: FFI · **심각도**: Medium · **탐지**: Structural

**나쁜 예**:
```rust
// bindgen이 생성한 그대로의 타입/함수를 그대로 pub re-export
pub use dav1d_sys::{Dav1dContext, Dav1dPicture, Dav1dSettings, dav1d_open, dav1d_get_picture};

// 애플리케이션 코드가 raw pointer와 unsafe 호출을 직접 다루게 됨
pub fn analyze_stream(path: &str) {
    let mut settings: dav1d_sys::Dav1dSettings = unsafe { mem::zeroed() };
    let mut ctx: *mut Dav1dContext = ptr::null_mut();
    unsafe { dav1d_sys::dav1d_open(&mut ctx, &settings) };
    // ... 이하 모든 애플리케이션 로직이 unsafe와 raw pointer로 뒤덮임
}
```

**문제**:
- bindgen이 생성하는 타입/함수는 C 헤더를 기계적으로 번역한 결과물로, Rust의 소유권/수명/에러 처리 관례를 전혀 반영하지 않는다 — 이를 그대로 도메인 API로 노출하면 이 카탈로그의 다른 모든 항목(null 가정, lifetime, ownership 불명확 등)이 애플리케이션 코드 전역으로 새어나간다
- unsafe 사용이 진입점(FFI wrapper 모듈) 한 곳에 갇히지 않고 호출부 전체로 퍼지면, "이 unsafe 블록이 안전한 이유"를 각 호출부마다 따로 논증해야 해서 검토 비용이 코드베이스 규모에 비례해 계속 증가한다
- 라이브러리 버전이 바뀌어 bindgen 출력이 달라지면(필드 추가/제거, 타입 변경) 그 변화가 애플리케이션 코드 전역에 컴파일 에러로 튀어 리팩터링 범위가 걷잡을 수 없이 커진다

**발생 조건**:
- 프로토타이핑 단계에서 "일단 동작하게" bindgen 크레이트를 직접 의존성에 추가하고 그대로 쓰다가, 안전 계층을 나중에 추가하겠다는 계획이 계속 미뤄질 때
- 안전 wrapper를 만드는 작업이 번거롭게 느껴져 "이 정도는 그냥 unsafe로 처리하지"라는 판단이 누적될 때

**권장**:
```rust
// dav1d_sys(bindgen 출력)는 crate 내부의 ffi 모듈에만 의존성으로 존재
mod ffi {
    pub(crate) use dav1d_sys::*;
}

// 공개 도메인 API는 안전한 타입/에러/수명만 노출
pub struct Decoder {
    ctx: NonNull<ffi::Dav1dContext>,
}

impl Decoder {
    pub fn open(settings: &DecoderSettings) -> Result<Self, DecodeError> { /* unsafe는 내부에 캡슐화 */ todo!() }
    pub fn decode(&mut self, data: &[u8]) -> Result<DecodedFrame, DecodeError> { todo!() }
}
// 애플리케이션 코드는 Decoder만 보고, unsafe/raw pointer는 전혀 보지 않는다
```
- bindgen 생성 크레이트는 항상 내부 전용 모듈(`ffi`, `sys` 등)에 격리하고 `pub`으로 재노출하지 않는다
- 안전한 도메인 타입(`Decoder`, `DecodedFrame`, `DecodeError`)을 별도로 설계해 unsafe 호출과 raw pointer 조작을 이 wrapper 계층 내부에만 가두고, 이 카탈로그의 다른 규칙들(FFI-001~018)을 이 경계에서 한 번만 제대로 적용한다
- wrapper 계층에 대한 단위 테스트를 두껍게 두어, "unsafe가 격리된 유일한 지점"의 신뢰도를 높인다

**탐지 방법**:
- 구조적 검사: `pub use <sys_crate>::*` 또는 bindgen 크레이트 타입이 `pub fn` 시그니처에 그대로 등장하는지 grep
- 아키텍처 검사: crate 의존성 그래프에서 bindgen 크레이트가 애플리케이션/도메인 레이어에서 직접 참조되는지 확인(가능하면 `cargo-deny`의 의존성 가시성 규칙 활용)
- 코드 리뷰: 새 C 라이브러리 바인딩 도입 시 "안전 wrapper 계층이 있는가"를 필수 항목으로 검토

**예외**:
- 매우 작고 내부용으로만 쓰이는 유틸리티(예: 사내 CLI 도구, 실험용 벤치마크 바이너리)로 외부에 노출되지 않고 unsafe 사용 범위가 한 파일에 국한된다면, 완전한 wrapper 계층 없이 진행하는 실용적 타협도 가능하다 — 다만 이 경우도 unsafe 사용처를 한 모듈로 모으는 최소한의 격리는 유지한다

**Bitvue 판정**: N/A — vvdec의 raw FFI 바인딩은 비공개 `mod ffi { ... }`(vvdec.rs:53)로 캡슐화되어 있고 `pub use vvdec::VvcDecoder`(lib.rs:37)만 외부에 노출됨; 애플리케이션 코드는 `Decoder` 트레이트(traits.rs)만 사용하고 raw pointer/`ffi::*` 타입을 직접 보지 않음. dav1d/libvmaf-rs/ffmpeg-next는 애초에 안전한 wrapper crate이므로 이 문제가 발생할 여지가 없음

---

### FFI-020: unsafe block이 너무 넓음
**분류**: FFI · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn process_frame(ctx: *mut Dav1dContext, frame_idx: usize, out: &mut Vec<f64>) -> Result<(), Error> {
    unsafe {
        let mut pic: Dav1dPicture = mem::zeroed();
        let ret = dav1d_get_picture(ctx, &mut pic);
        if ret != 0 {
            return Err(Error::Decode(ret));
        }
        let width = pic.p.w as usize;
        let height = pic.p.h as usize;
        let stride = pic.stride[0] as usize;
        let mut histogram = vec![0u32; 256];
        // 아래 40줄은 전부 안전한 순수 Rust 로직(히스토그램 계산, 정규화, 통계 산출)인데
        // 전체가 unsafe 블록 안에 있어 "정말 unsafe가 필요한 부분"이 어디인지 알 수 없음
        for row in 0..height {
            let row_ptr = (pic.data[0] as *const u8).add(row * stride);
            let row_slice = slice::from_raw_parts(row_ptr, width);
            for &v in row_slice {
                histogram[v as usize] += 1;
            }
        }
        let total = (width * height) as f64;
        for count in &histogram {
            out.push(*count as f64 / total);
        }
        Ok(())
    }
}
```

**문제**:
- unsafe 블록의 목적은 "여기서부터 여기까지는 컴파일러가 검증하지 못하니 사람이 특별히 주의 깊게 검토해야 한다"는 신호인데, 블록이 너무 넓으면 이 신호가 무의미해진다 — 리뷰어가 "이 40줄 전체가 unsafe 불변조건과 관련 있는가"를 매번 처음부터 다시 판단해야 한다
- 안전한 순수 로직(히스토그램 집계, 정규화)이 unsafe 블록 안에 있으면, 나중에 이 로직을 수정하는 사람이 "여기 unsafe가 왜 필요하지?"라고 오해하거나, 반대로 안전한 리팩터링(예: 이 로직을 별도 함수로 추출)이 "unsafe 블록을 건드리는 변경"으로 취급되어 불필요하게 신중한 리뷰를 유발한다
- 진짜 unsafe 불변조건(포인터 유효성, stride 계산, 슬라이스 길이)이 안전한 로직과 뒤섞여 있으면, 그 불변조건이 실제로 어디서 성립하고 어디서 깨질 수 있는지 추적하기 어려워져 버그 삽입 지점이 늘어난다

**발생 조건**:
- 함수 하나가 "raw pointer에서 데이터를 꺼내는 것"과 "그 데이터로 계산하는 것"을 함께 수행할 때, 편의상 함수 전체를 `unsafe fn`이나 최상위 `unsafe { }`로 감싸는 습관
- unsafe 블록을 최소화하는 리팩터링이 "나중에 시간 날 때" 항목으로 밀려 누적될 때

**권장**:
```rust
fn process_frame(ctx: *mut Dav1dContext, out: &mut Vec<f64>) -> Result<(), Error> {
    let pic = get_picture(ctx)?; // unsafe는 이 함수 내부에 캡슐화
    let luma: &[u8] = luma_plane(&pic); // 여기도 unsafe는 내부에 국한, 반환은 안전한 &[u8]

    // 이 아래로는 순수 안전 Rust — unsafe 블록이 전혀 없다
    let mut histogram = [0u32; 256];
    for &v in luma {
        histogram[v as usize] += 1;
    }
    let total = luma.len() as f64;
    out.extend(histogram.iter().map(|&c| c as f64 / total));
    Ok(())
}

fn get_picture(ctx: *mut Dav1dContext) -> Result<Dav1dPicture, Error> {
    let mut pic: Dav1dPicture = unsafe { mem::zeroed() };
    let ret = unsafe { dav1d_get_picture(ctx, &mut pic) };
    if ret != 0 { return Err(Error::Decode(ret)); }
    Ok(pic)
}

fn luma_plane(pic: &Dav1dPicture) -> &[u8] {
    let width = pic.p.w as usize;
    let height = pic.p.h as usize;
    let stride = pic.stride[0] as usize;
    // 이 함수 안에서만 unsafe, row 단위 stride 처리(FFI-005 참고)까지 캡슐화
    unsafe { slice::from_raw_parts(pic.data[0] as *const u8, stride * height) }
        .chunks(stride).take(height).flat_map(|row| &row[..width])
        .copied().collect::<Vec<_>>().leak() // 예시 단순화, 실제로는 소유 버퍼 권장
}
```
- unsafe 블록/함수는 "raw pointer, transmute, FFI 호출 등 컴파일러가 검증할 수 없는 연산"만 최소 범위로 감싸고, 그 결과를 안전한 타입(`&[u8]`, `Result<T, E>`)으로 즉시 변환해 반환한다
- 안전 계층(히스토그램 계산 등 순수 로직)은 별도 함수로 분리해 unsafe와 물리적으로 떨어뜨린다
- `unsafe fn`을 정의할 때도 함수 시그니처에 안전 불변조건(safety invariant)을 doc comment(`/// # Safety`)로 명시한다

**탐지 방법**:
- clippy `unsafe_derive_deserialize`, `missing_safety_doc` 등 관련 lint 활성화, `#![deny(unsafe_op_in_unsafe_fn)]`로 unsafe 연산 각각을 명시적으로 표시하도록 강제
- 구조적 검사: unsafe 블록의 라인 수를 측정하는 스크립트(예: 20줄 초과 unsafe 블록을 CI 경고로)
- 코드 리뷰: unsafe 블록을 리뷰할 때 "이 블록 안의 각 줄이 실제로 unsafe 연산과 관련 있는가"를 줄 단위로 확인

**예외**:
- unsafe 연산들이 서로 강하게 얽혀 있어(예: 포인터 계산 결과가 바로 다음 unsafe 호출의 불변조건이 되는 경우) 분리하면 오히려 불변조건을 눈으로 추적하기 더 어려워지는 경우는, 블록을 유지하되 각 unsafe 연산 앞에 불변조건을 설명하는 주석을 촘촘히 남기는 것으로 대체할 수 있다

**Bitvue 판정**: Confirmed — `convert_frame`(vvdec.rs:372-462) 전체가 단일 `unsafe { }` 블록으로 감싸여 있고, 그 안에 frame_type 매칭(426-433)·chroma_format 계산(438-444)·`DecodedFrame` 구조체 생성(446-460) 같은 순수 안전 로직이 실제 unsafe 연산(`&*frame` 역참조, raw pointer 기반 plane 추출)과 물리적으로 분리되지 않고 섞여 있음

---

### FFI-021: 동적 링킹 시점 ABI skew (컴파일 헤더와 런타임 라이브러리 버전 불일치)
**분류**: FFI · **심각도**: Critical · **탐지**: Manual

**나쁜 예**:
```rust
// build.rs / Cargo.toml
// dav1d-sys가 bindgen으로 시스템 헤더(예: dav1d 1.2 버전)를 파싱해 struct 레이아웃을 고정 생성

// 배포 환경(Docker 베이스 이미지, 사용자 시스템 패키지)에는 dav1d 1.4 버전 .so가 설치되어 있고,
// 그 사이에 Dav1dPicture에 필드가 추가/재배치됨
extern "C" {
    fn dav1d_get_picture(ctx: *mut Dav1dContext, out: *mut Dav1dPicture) -> c_int;
}
// 링크는 동적으로 이루어지므로 컴파일은 성공하지만, 런타임에 채워지는 struct의
// 실제 필드 레이아웃이 컴파일 시점에 가정한 레이아웃과 다를 수 있음
```

**문제**:
- bindgen은 "빌드 시점에 참조한 헤더"를 기준으로 `#[repr(C)]` struct 레이아웃(필드 순서, 크기, 정렬)을 고정해서 Rust 코드를 생성하는데, 동적 링킹(`.so`/`.dylib`/`.dll`)은 이 레이아웃을 런타임에 재검증하지 않는다 — 컴파일 시점 헤더와 실행 시점 실제 라이브러리 바이너리의 ABI가 어긋나면, struct의 필드를 완전히 잘못된 오프셋에서 읽고 쓰게 되는 조용한 메모리 손상이 발생
- 이 문제는 정적 링킹이나 vendored 빌드(라이브러리 소스를 프로젝트에 포함해 항상 같은 버전으로 빌드)에서는 발생하지 않고, "시스템에 설치된 공유 라이브러리에 동적으로 링크"하는 배포 방식(Docker 베이스 이미지의 apt/dnf 패키지, 사용자 로컬 환경의 brew/pacman 패키지)에서만 나타나 재현이 매우 환경 의존적이다
- 특히 SONAME/버전 정책이 느슨한 라이브러리(메이저 버전을 안 올리고 struct에 필드를 추가하는 등 ABI 호환성 규율이 약한 프로젝트)를 다룰 때 위험이 커지며, dav1d/libvmaf 모두 활발히 개발 중인 비교적 젊은 프로젝트라 이런 변화가 상대적으로 빈번할 수 있다

**발생 조건**:
- CI에서 빌드한 바이너리를 다른 시스템(다른 버전의 라이브러리가 설치된 Docker 이미지, 사용자 머신)에 배포하면서 동적 링킹을 사용할 때
- `pkg-config`나 시스템 헤더 검색으로 빌드 시점 라이브러리 버전을 고정하지 않고, "적당히 설치되어 있는 버전"에 의존할 때
- 컨테이너 베이스 이미지를 업데이트했는데 애플리케이션 바이너리는 재빌드하지 않고 재사용할 때(가장 흔한 실제 사고 시나리오)

**권장**:
```rust
// build.rs: 빌드 시점에 실제로 링크될 라이브러리의 버전을 pkg-config로 명시적으로 고정하고 기록
fn main() {
    let lib = pkg_config::Config::new()
        .atleast_version("1.4.0")
        .exactly_version("1.4.2") // 가능하면 exact pin, 최소한 semver 범위를 명시
        .probe("dav1d")
        .expect("dav1d 1.4.2 not found via pkg-config");
    println!("cargo:warning=linked dav1d version: {:?}", lib.version);
}
```
- 가능하면 정적 링킹(vendored 소스 빌드) 또는 컨테이너 이미지에 라이브러리를 애플리케이션과 함께 고정 버전으로 번들링해 "빌드 시점 헤더 == 런타임 바이너리" 등식을 물리적으로 보장한다
- 동적 링킹이 불가피하다면 `pkg-config`로 최소/정확 버전을 빌드 시점에 강제하고, 배포 스크립트/Dockerfile에서 동일 버전을 고정(`apt-get install libdav1d-dev=1.4.2-1`처럼 버전 핀)한다
- 런타임 자체 점검: 라이브러리가 버전 조회 API(`dav1d_version()` 등)를 제공한다면, 애플리케이션 시작 시 그 버전을 확인해 예상 범위를 벗어나면 명시적으로 에러를 내고 종료(조용한 메모리 손상보다 명시적 실패가 낫다)

**탐지 방법**:
- 배포 파이프라인 감사: CI 빌드 환경과 프로덕션/배포 대상 환경의 라이브러리 버전이 실제로 동일한지 확인하는 배포 전 체크 스크립트(`ldd`/`otool -L`로 실제 링크된 `.so`/`.dylib` 버전 비교)
- Runtime: 애플리케이션 시작 시 라이브러리 버전 API 호출 결과를 로그로 남기고, 알려진 지원 버전 목록과 대조하는 자체 검사
- Manual: 의존성 업데이트/배포 환경 변경 시마다 "빌드 시점 헤더 버전과 배포 대상 라이브러리 버전이 일치하는가"를 릴리스 체크리스트 항목으로 명시

**예외**:
- 완전히 정적 링킹된 바이너리(라이브러리 소스가 vendored되어 프로젝트와 함께 빌드됨)는 이 문제에서 원천적으로 자유롭다 — 가능하다면 이것이 가장 확실한 예방책이다

**Bitvue 판정**: Confirmed — vvdec.rs:157 `#[link(name = "vvdec")]`로 시스템에 설치된 공유 라이브러리에 동적 링크하면서 `pkg-config` 버전 고정이나 exact-version 강제가 전혀 없음; `bitvue-decode/Cargo.toml:27-29` 주석도 "brew install vvdec" / "Build from source"처럼 버전 미고정 설치를 전제로 함. `vvdec_get_version()` 함수는 존재하지만(vvdec.rs:177) 테스트에서만 호출되고(vvdec.rs:934-947) `VvcDecoder::new()` 초기화 경로(vvdec.rs:321-369)에는 런타임 버전 검증이 없음

---

### FFI-022: repr(Rust) 구조체를 FFI 경계에 그대로 사용
**분류**: FFI · **심각도**: High · **탐지**: Static

**나쁜 예**:
```rust
// #[repr(C)] 없이 기본 Rust 레이아웃(컴파일러가 필드 순서/패딩을 임의로 최적화)
struct FrameMeta {
    width: u32,
    height: u32,
    pts: i64,
    flags: u8,
}

extern "C" {
    // C 콜백이 FrameMeta*를 받아 필드를 순서대로 읽는다고 가정
    fn vmaf_register_frame_callback(cb: extern "C" fn(*const FrameMeta), cookie: *mut c_void);
}

extern "C" fn on_frame(meta: *const FrameMeta) {
    // C 쪽 코드(또는 다른 컴파일 단위)가 이 struct를 width/height/pts/flags 순서의
    // C 레이아웃으로 이해하고 있다면, Rust 기본 레이아웃과 어긋날 수 있음
}
```

**문제**:
- Rust의 기본 struct 레이아웃(`repr(Rust)`)은 "안정된 레이아웃을 보장하지 않는다"고 명시적으로 문서화되어 있다 — 필드 순서 재배치, 패딩 삽입/제거가 컴파일러 버전이나 최적화 옵션에 따라 달라질 수 있으며, 심지어 같은 컴파일러의 다른 빌드에서도 달라질 수 있다
- 이런 타입을 C 콜백 시그니처의 포인터 대상으로 쓰거나, C 쪽에서도 동일한 필드 순서를 가정하는 struct로 캐스팅해 사용하면, 두 쪽이 서로 다른 메모리 레이아웃을 가정한 채 같은 바이트를 다른 필드로 해석하는 UB가 발생 — 이는 대부분의 경우 컴파일이 성공하고 링크도 성공하기 때문에 정적으로 전혀 드러나지 않는다
- 특히 이 구조체를 값으로 반환하거나(`extern "C" fn() -> FrameMeta`), C 쪽 struct 정의와 대응시켜야 하는 모든 지점에서 문제가 되며, 단순히 opaque 포인터로만 주고받는 핸들 타입에는 해당하지 않는다는 점에서 FFI-019(bindgen 노출)와는 다른 축의 문제다

**발생 조건**:
- Rust에서 정의한 struct를 C 콜백 시그니처의 인자/반환 타입으로 사용하거나, C 헤더에 대응하는 struct 정의가 있는 상황에서 Rust 쪽에 `#[repr(C)]`를 깜빡 누락할 때
- 순수 Rust ↔ Rust 통신용으로 만든 struct를 나중에 FFI 경계로 재사용하면서 `#[repr(C)]` 추가를 잊을 때(리팩터링 중 발생하기 쉬움)

**권장**:
```rust
#[repr(C)] // C와 동일한 필드 순서/정렬 규칙을 명시적으로 고정
struct FrameMeta {
    width: u32,
    height: u32,
    pts: i64,
    flags: u8,
    _pad: [u8; 7], // C 쪽 struct의 패딩과 명시적으로 맞춰 정렬 불일치를 방지
}

extern "C" fn on_frame(meta: *const FrameMeta) {
    // 이제 레이아웃이 C ABI와 일치함을 컴파일러가 보장
}
```
- FFI 경계(콜백 시그니처, extern 함수의 인자/반환 타입, C 헤더와 대응하는 모든 struct)에 노출되는 모든 Rust struct에는 예외 없이 `#[repr(C)]`를 붙인다
- 가능하면 `cbindgen`으로 Rust struct에서 C 헤더를 자동 생성해, 두 언어의 struct 정의가 항상 하나의 소스(Rust 쪽 정의)에서 파생되도록 해 수동 동기화 실수를 원천 차단한다
- CI에 "FFI 경계에 노출되는 struct 목록"을 정적으로 추출해 `#[repr(C)]` 누락을 검사하는 lint/스크립트를 둔다

**탐지 방법**:
- clippy에는 이를 직접 잡는 lint가 약하므로, 구조적 검사로 `extern "C" fn`/`extern "C" { }` 시그니처에 등장하는 struct 타입 목록을 추출해 각각 `#[repr(C)]` 여부를 자동 확인하는 스크립트 작성
- `cbindgen --verify` 같은 도구로 Rust struct에서 생성한 헤더와 실제 C 쪽 헤더 정의가 일치하는지 CI에서 자동 검증
- Miri는 단일 프로세스 내 레이아웃 불일치까지는 잡지 못할 수 있으므로, 실제 C 컴파일러로 생성한 헤더와의 대조(정적 검사)가 더 실효적이다

**예외**:
- struct가 순수하게 Rust ↔ Rust 통신에만 쓰이고 FFI 경계를 전혀 넘지 않는다면 `repr(Rust)` 기본값이 오히려 컴파일러 최적화(필드 재배치로 패딩 최소화) 혜택을 준다 — `#[repr(C)]`를 FFI와 무관한 모든 struct에 습관적으로 붙이는 것은 이 카테고리의 반대 방향 안티패턴(불필요한 제약)이므로, 정말 경계를 넘는 타입에만 선택적으로 적용한다

**Bitvue 판정**: N/A — vvdec ffi 모듈의 모든 FFI 경계 구조체(`VvdecAccessUnit`, `VvdecPlane`, `VvdecComponentType`, `VvdecFrameType`, `VvdecColorFormat`, `VvdecFrame`, `VvdecParams`)에 `#[repr(C)]`가 정확히 적용되어 있음(vvdec.rs:61-141); 콜백 함수 포인터도 없음(FFI-009 참고)
