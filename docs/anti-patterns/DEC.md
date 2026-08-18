# Anti-Pattern Catalog — DEC: Decode·FFmpeg 통합

이 문서는 Bitvue류(Tauri + Rust + React 기반, AV1/HEVC/AVC/VP9/VVC/AV3/MPEG-2 파서, dav1d AV1 디코드, FFmpeg 스타일 하드웨어/소프트웨어 디코더 통합을 포함하는) 비디오 비트스트림 분석기를 위한 안티패턴 카탈로그의 일부입니다(Phase 4, Wave 4). 전체 목차는 `docs/anti-patterns/INDEX.md`를 참고하십시오. 이 파일은 1단계(일반 카탈로그 작성) 산출물이며, 2단계에서 실제 Bitvue 저장소를 이 기준으로 감사합니다.

**FFI.md와의 관계**: FFI는 Rust↔C 경계에서의 메모리 안전성(포인터 수명, 소유권, unsafe 계약)을 다루고, DEC는 그 경계 너머 디코더의 상태 머신·타임스탬프·프레임 순서·메타데이터 같은 "의미론적" 정확성을 다룬다. 같은 `avcodec_send_packet` 호출이라도 FFI는 "포인터가 유효한가"를, DEC는 "이 호출을 올바른 순서·조건으로 했는가"를 본다.

---

### DEC-001: send_packet/receive_frame 상태 머신 오해
**분류**: DEC · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
// avcodec_send_packet / avcodec_receive_frame을 1:1 대응으로 오해
unsafe fn decode_packet(ctx: *mut AVCodecContext, pkt: *mut AVPacket) -> Option<Frame> {
    let ret = avcodec_send_packet(ctx, pkt);
    if ret < 0 {
        return None;
    }
    let frame = av_frame_alloc();
    let ret = avcodec_receive_frame(ctx, frame);
    if ret < 0 {
        av_frame_free(&mut (frame as *mut _));
        return None; // EAGAIN도 여기서 그냥 실패로 처리됨
    }
    Some(Frame::from_raw(frame))
}

fn decode_all(ctx: *mut AVCodecContext, packets: &[Packet]) -> Vec<Frame> {
    packets.iter().filter_map(|p| unsafe { decode_packet(ctx, p.as_raw()) }).collect()
}
```

**문제**:
- `send_packet` 한 번에 `receive_frame`이 정확히 한 번 성공한다는 가정은 틀렸다. 디코더는 내부 버퍼(B-frame reorder, lookahead)를 가지고 있어 패킷 하나를 넣어도 프레임이 0개 나올 수도, 여러 개(멀티 프레임 패킷, 일부 레거시 코덱) 나올 수도 있다.
- `packets.len() != frames.len()`이 항상 성립한다고 가정하면 디코드 결과가 조용히 누락되거나 순서가 뒤섞인다.
- `filter_map`으로 실패를 흡수하면 "이 프레임은 아직 안 나왔을 뿐"과 "진짜 디코드 에러"를 구분하지 못한 채 손실이 발생한다.

**발생 조건**:
- B-frame을 사용하는 GOP 구조(HEVC/AVC hierarchical B, AV1 altref reorder)에서 특히 두드러진다.
- 컨테이너가 프레임당 패킷 1개를 보장하지 않는 경우(레거시 mux, 일부 raw stream)에 멀티 프레임 패킷이 섞여 있으면 더 심해진다.

**권장**:
```rust
unsafe fn decode_packet(ctx: *mut AVCodecContext, pkt: *mut AVPacket, out: &mut Vec<Frame>) -> Result<(), DecodeError> {
    let mut ret = avcodec_send_packet(ctx, pkt);
    if ret < 0 && ret != AVERROR(EAGAIN) {
        return Err(DecodeError::from_averror(ret));
    }
    loop {
        let frame = av_frame_alloc();
        ret = avcodec_receive_frame(ctx, frame);
        if ret == AVERROR(EAGAIN) || ret == AVERROR_EOF {
            av_frame_free(&mut (frame as *mut _));
            break; // 정상: 더 넣을 패킷이 필요하거나 스트림 끝
        } else if ret < 0 {
            av_frame_free(&mut (frame as *mut _));
            return Err(DecodeError::from_averror(ret));
        }
        out.push(Frame::from_raw(frame)); // 프레임 0개~N개 모두 정상 케이스
    }
    Ok(())
}
```
- send_packet 한 번당 receive_frame을 EAGAIN/EOF가 나올 때까지 루프로 소진한다.
- 패킷 개수와 프레임 개수를 동일시하지 않는다. 총 프레임 수는 flush 이후에만 확정된다.
- 진짜 에러(음수, EAGAIN/EOF 제외)와 "정상적으로 더 필요함"을 코드로 구분한다.

**탐지 방법**:
- Structural: `avcodec_send_packet` 호출 직후 `avcodec_receive_frame`이 루프 없이 정확히 한 번만 호출되는 패턴을 AST/제어흐름 분석으로 탐지.
- Static: `while`/`loop` 없이 `receive_frame`을 호출하는 함수 시그니처 스캔.
- Runtime: 입력 패킷 수와 출력 프레임 수가 다른 테스트 스트림(B-frame 다수)으로 회귀 테스트.

**예외**:
- 진짜 프레임 단위 무손실 인코딩이나 all-intra(모든 프레임 keyframe, no reorder, no delay) 스트림에서는 실질적으로 1:1에 가깝지만, 그래도 루프 형태를 유지하는 것이 안전하며 성능 비용은 무시할 만하다.

**Bitvue 판정**: N/A — (재검증, 이전 판정은 stale) 이전 Confirmed 근거였던 `decode_ivf_frame_generic`(src-tauri/src/commands/frame.rs)는 Tauri→Electron 전환의 일부로 `src-tauri` 자체가 삭제되며(`e7194cc`, 현재 HEAD의 조상 커밋) 더 이상 존재하지 않는다. 그 자리를 대체한 현재 커맨드들 — `get_decoded_frame_yuv`/`get_thumbnails`(crates/bitvue-sidecar/src/decode_bridge.rs:52-59,193-203), `decode_av1_luma`(crates/bitvue-cli/src/commands/decode.rs:815-817), `decode_ivf_frames`(crates/bitvue-cli/src/commands/quality.rs:200-213), `find_first_diff` 계열(crates/bitvue-sidecar/src/debug_yuv.rs:637-641), `decode_ivf`/`decode_ivf_streaming`(crates/bitvue-decode/src/decoder.rs:761-764,805-808) — 는 전부 `send_data_owned` 1회 뒤 `while let Ok(frame) = dec.get_frame()`로 EAGAIN까지 소진하는 루프를 쓴다. `FfmpegDecoder::decode_packet`(crates/bitvue-decode/src/ffmpeg.rs:114-151)도 여전히 올바른 루프 패턴. 코드베이스 전체에서 1패킷=1프레임 가정 위반 사례를 찾지 못했다.

---

### DEC-002: EAGAIN을 fatal error로 처리
**분류**: DEC · **심각도**: Critical · **탐지**: Static

**나쁜 예**:
```rust
unsafe fn send_and_check(ctx: *mut AVCodecContext, pkt: *mut AVPacket) -> Result<(), String> {
    let ret = avcodec_send_packet(ctx, pkt);
    if ret < 0 {
        // EAGAIN(디코더 내부 버퍼 포화, receive_frame 먼저 호출 필요)도
        // 다른 모든 에러코드와 동일하게 fatal로 처리됨
        return Err(format!("send_packet failed: {ret}"));
    }
    Ok(())
}
```

**문제**:
- `AVERROR(EAGAIN)`은 "지금은 더 못 받으니 `receive_frame`을 먼저 호출해 버퍼를 비워라"라는 흐름 제어 신호이지 에러가 아니다.
- 이를 fatal로 처리하면 디코더 내부 버퍼가 찬 시점(보통 B-frame reorder depth 근처)마다 디코드가 중단되고, 파일 앞부분 몇 프레임만 분석된 채 나머지는 "에러"로 보고된다.
- 사용자에게는 "이 파일은 디코드가 안 된다"는 잘못된 진단이 나가고, 실제로는 정상 스트림이다.

**발생 조건**:
- send_packet과 receive_frame을 번갈아 호출하지 않고 send만 반복 호출하는 루프 구조에서 필연적으로 발생.
- 디코더 lookahead/버퍼 depth가 큰 코덱(HEVC hierarchical GOP, AV1 다중 레퍼런스)일수록 빨리 발현.

**권장**:
```rust
unsafe fn send_packet_retrying(
    ctx: *mut AVCodecContext,
    pkt: *mut AVPacket,
    drain: impl Fn() -> Result<(), DecodeError>,
) -> Result<(), DecodeError> {
    loop {
        let ret = avcodec_send_packet(ctx, pkt);
        match ret {
            0 => return Ok(()),
            e if e == AVERROR(EAGAIN) => {
                drain()?; // receive_frame을 먼저 소진하고 재시도
                continue;
            }
            e => return Err(DecodeError::from_averror(e)),
        }
    }
}
```
- `EAGAIN`/`EOF`는 전용 분기로 처리하고, 나머지 음수만 fatal 에러로 승격한다.
- send/receive를 상호 재시도 가능한 루프로 묶어 "버퍼가 찼으면 비우고 다시 보낸다"를 명시적으로 구현한다.

**탐지 방법**:
- Static: `avcodec_send_packet`/`avcodec_receive_packet` 반환값 검사 코드에서 `EAGAIN` 상수 매칭이 없는 `if ret < 0 { return Err }` 패턴 grep.
- Runtime: 디코더 버퍼가 찰 만큼 긴(수백 프레임) 테스트 벡터로 실제 실패 여부 회귀 테스트.

**예외**:
- 완전히 동기적인 stateless 유틸(예: 단일 프레임 썸네일 디코더로 GOP 하나만 처리)에서는 EAGAIN이 실질적으로 발생하지 않을 수 있으나, 그래도 방어적으로 분기해두는 비용은 거의 없다.

**Bitvue 판정**: Confirmed — `Av1Decoder::send_data_owned`(crates/bitvue-decode/src/decoder.rs:331-337)와 `FfmpegDecoder::decode_packet`의 `send_packet` 호출(crates/bitvue-decode/src/ffmpeg.rs:116-118) 모두 반환값을 `.map_err`로 그대로 fatal `DecodeError`로 승격시키며, EAGAIN을 별도 분기하거나 재시도(drain 후 재전송)하는 코드가 크레이트 전체에 없다. 다만 실사용 경로는 전부 "send 1회 → get_frame을 EAGAIN까지 소진" 순서로 매 반복마다 드레인하므로 실제로 EAGAIN이 send 단계에서 터질 조건(드레인 안 된 채 연속 send)은 거의 발생하지 않아 이론적 결함에 가깝다. 또한 `FfmpegDecoder`(H264/HEVC/VP9 픽셀 디코드)는 `crates/bitvue-decode/src/lib.rs:34`에서 재노출만 될 뿐 sidecar/cli 등 하위 크레이트 어디에서도 호출되지 않는 죽은 코드로 확인됨(grep 결과 0건) — 현재 실도달 가능한 것은 `Av1Decoder` 경로뿐.

---

### DEC-003: flush packet 누락
**분류**: DEC · **심각도**: High · **탐지**: Structural

**나쁜 예**:
```rust
fn decode_stream(ctx: *mut AVCodecContext, packets: &[Packet]) -> Vec<Frame> {
    let mut frames = Vec::new();
    for pkt in packets {
        unsafe { decode_packet_loop(ctx, pkt.as_raw(), &mut frames) };
    }
    // 스트림 끝에서 flush(NULL packet) 없이 그대로 반환
    frames
}
```

**문제**:
- 디코더가 내부에 들고 있던 지연 프레임(B-frame reorder buffer에 갇힌 마지막 GOP)이 한 번도 방출되지 않는다.
- `send_packet(NULL)`(flush 신호) 없이는 디코더가 "더 이상 입력이 없다"는 것을 모르고 계속 대기 상태로 남는다.
- 전체 프레임 수 카운트, 마지막 몇 프레임의 메트릭(PSNR/VMAF), duration 계산이 실제 스트림보다 짧게 나와 통계가 틀어진다.

**발생 조건**:
- 파일 끝(EOF)에서 흔히 발생. reorder depth가 클수록(예: HEVC B-pyramid depth 4) 유실 프레임 수도 커진다.
- 세그먼트 단위 분석(구간 잘라서 디코드)에서 구간 끝마다 flush를 빼먹으면 매 세그먼트마다 손실이 누적된다.

**권장**:
```rust
fn decode_stream(ctx: *mut AVCodecContext, packets: &[Packet]) -> Vec<Frame> {
    let mut frames = Vec::new();
    for pkt in packets {
        unsafe { decode_packet_loop(ctx, pkt.as_raw(), &mut frames) };
    }
    unsafe {
        // NULL 패킷 = flush 신호: 디코더가 EOF에 도달했음을 명시
        avcodec_send_packet(ctx, std::ptr::null());
        drain_remaining_frames(ctx, &mut frames); // EOF가 나올 때까지 receive_frame 반복
    }
    frames
}
```
- EOF에서 반드시 `send_packet(NULL)`(FFmpeg) 혹은 해당 디코더 API의 flush 신호(dav1d의 `dav1d_send_data`에 EOF 전달 후 drain)를 호출한다.
- flush 이후 `receive_frame`을 `AVERROR_EOF`가 나올 때까지 계속 호출해 마지막 프레임까지 모두 뽑아낸다.
- 세그먼트 단위 디코드라면 세그먼트 경계마다 이 흐름을 반복하도록 명시적 함수로 캡슐화한다.

**탐지 방법**:
- Structural: 디코드 루프 종료 지점(파일 끝, 함수 반환 직전) 도달 경로 중 flush 호출로 이어지지 않는 경로를 제어흐름 그래프로 탐지.
- Runtime: known frame count 테스트 벡터를 디코드해 `decoded_frames == expected_frames`인지 회귀 테스트.

**예외**:
- reorder delay가 0인 구성(all-intra, low-delay B 없음)에서는 flush 누락의 영향이 없거나 미미할 수 있지만, 코덱/설정이 바뀌면 조용히 재발하므로 항상 flush를 호출하는 편이 안전하다.

**Bitvue 판정**: N/A — (재검증, 이전 판정은 stale) 근거였던 src-tauri/src/commands/quality.rs 전체가 `src-tauri` 삭제(`e7194cc`)로 더 이상 존재하지 않는다. 현재 코드는 정반대로 일관되게 올바르다: `get_decoded_frame_yuv`(decode_bridge.rs:60-67), `get_thumbnails`(decode_bridge.rs:204-214), `decode_av1_luma`(cli/decode.rs:817-823), `decode_ivf_frames`(cli/quality.rs:216-221), `find_first_diff`(debug_yuv.rs:648-651) 전부 EOF에서 `flush()` 대신 명시적으로 `drain_decoder_frames()`를 호출하며, 각 파일에 "flush()는 seek용으로 내부 상태를 지울 뿐 버퍼링된 프레임을 비우지 않아 짧은 스트림에서 프레임이 누락됐었다"는 동일한 경고 주석이 반복된다(2026-08-10 dav1d flush() 버그 수정의 정착된 패턴). placeholder/회색 프레임 합성 코드는 grep 결과 0건 — DEC-016 참고.

---

### DEC-004: decoder delay 프레임 누락
**분류**: DEC · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn frame_count_matches_gop(ctx: *mut AVCodecContext, packets: &[Packet]) -> bool {
    // "패킷을 넣으면 그 즉시 대응 프레임이 나온다"는 잘못된 멘탈 모델로 설계된 API
    let mut n = 0;
    for pkt in packets {
        if let Some(_frame) = unsafe { decode_packet(ctx, pkt.as_raw()) } {
            n += 1;
        }
        // receive_frame이 EAGAIN이면 그냥 "이 패킷에 대응하는 프레임 없음"으로 스킵
    }
    n == packets.len()
}
```

**문제**:
- DEC-001/003과 근본 원인은 같지만 여기서는 "디코더 지연(delay)"이라는 개념 자체를 시스템 설계에서 빼먹은 경우를 별도로 짚는다: 디코더 초기화 파라미터(`ctx.delay` / `has_b_frames`)를 조회하지 않고 지연이 0이라고 암묵 가정한다.
- 진행률 표시줄, 프레임 인덱스 매핑, 실시간 미리보기 등에서 "N번째 패킷을 넣었으니 N번째 프레임이 나왔겠지"라는 오프바이-N 오류가 스트림 전체에 걸쳐 누적된다.
- 디코더 delay를 조회하지 않으면 애초에 몇 프레임을 버퍼링해야 UI가 프레임을 표시하기 시작해도 되는지 알 수 없다.

**발생 조건**:
- reorder delay가 있는 모든 스트림. 특히 인코더가 큰 GOP·다중 레퍼런스 구조를 쓸 때(AV1 altref, HEVC RPS로 4~8 프레임 지연) 심해짐.
- "패킷 N번째 = 프레임 N번째"라는 가정이 코드 여러 곳(진행률 계산, seek 대상 인덱스, 타임라인 UI)에 흩어져 있으면 한 곳만 고치고 다른 곳은 남아 재발한다.

**권장**:
```rust
unsafe fn expected_output_delay(ctx: *const AVCodecContext) -> i32 {
    (*ctx).delay // AVCodecContext.delay: 디코더가 내부에 보유할 수 있는 최대 프레임 수
}

fn decode_with_delay_awareness(ctx: *mut AVCodecContext, packets: &[Packet]) -> Vec<Frame> {
    let delay = unsafe { expected_output_delay(ctx) };
    let mut frames = Vec::new();
    for pkt in packets {
        unsafe { decode_packet_loop(ctx, pkt.as_raw(), &mut frames) };
    }
    unsafe { flush_and_drain(ctx, &mut frames) };
    debug_assert!(
        frames.len() as i32 >= packets.len() as i32 - delay,
        "delay={delay} 프레임을 고려해도 예상보다 많은 프레임이 누락됨"
    );
    frames
}
```
- 디코더가 보고하는 delay(예: `AVCodecContext.delay`, dav1d의 `max_frame_delay`)를 조회해 "패킷 수 = 프레임 수"가 아니라 "패킷 수 - delay ≤ 누적 프레임 수 ≤ 패킷 수"라는 관계로 설계한다.
- 진행률·프레임 인덱스 매핑은 패킷 인덱스가 아니라 실제로 방출된 프레임의 타임스탬프/카운트를 기준으로 한다.

**탐지 방법**:
- Semantic: 패킷 인덱스를 프레임 인덱스로 직접 사용하는 변수 흐름을 데이터플로 분석으로 추적.
- Runtime: 큰 reorder delay를 가진 합성 스트림(GOP 구조를 알고 있는 테스트 벡터)으로 방출된 프레임 수와 타임스탬프 순서를 검증.

**예외**:
- delay가 0으로 보고되는 구성(저지연 인코딩 프로파일)에서는 패킷=프레임 가정이 실제로 성립하지만, 이는 디코더가 보고한 값을 확인한 결과여야지 임의 가정이어서는 안 된다.

**Bitvue 판정**: N/A — (재검증, 이전 판정은 stale) 근거였던 `decode_container_h26x_frame_yuv`/`decode_annexb_frame_yuv`(src-tauri/src/commands/frame.rs)는 `src-tauri` 삭제(`e7194cc`)로 더 이상 존재하지 않는다. 현재 `get_decoded_frame_yuv`(crates/bitvue-sidecar/src/decode_bridge.rs:40-77)는 "마지막으로 나온 프레임"이 아니라, 스트림 처음부터 방출되는 모든 프레임을 순서대로 `decoded: Vec<DecodedFrame>`에 누적하고 그중 `decoded[frame_index]`를 반환한다(60-74행) — 패킷 인덱스가 아니라 실제 출력 순번으로 인덱싱하므로 이 안티패턴이 기술한 오프바이 문제가 구조적으로 발생하지 않는다.

---

### DEC-005: packet timestamp를 frame timestamp로 그대로 사용
**분류**: DEC · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn attach_timestamp(pkt: &Packet, frame: &mut Frame) {
    // 디코드 순서(패킷)의 pts를 표시 순서(프레임)에 그대로 복사
    frame.pts = pkt.pts;
}
```

**문제**:
- B-frame reorder가 있는 스트림에서는 패킷이 디코더에 들어가는 순서(decode order)와 프레임이 디코더에서 나오는 순서(presentation order)가 다르다. 하지만 프레임과 그에 대응하는 pts는 디코더 내부에서 이미 순서가 맞춰져 나온다.
- 마지막으로 `send_packet`한 패킷의 pts를 그다음에 `receive_frame`한 프레임에 그냥 갖다 붙이면, 그 프레임은 실제로는 몇 프레임 전에 디코드 큐에 들어간 다른 패킷에 대응하는 프레임일 수 있다.
- 결과적으로 타임라인 UI의 프레임 순서, seek 타겟 매칭, duration 계산이 모두 어긋난다.

**발생 조건**:
- B-frame이 있는 모든 실사용 스트림(대부분의 방송/스트리밍 콘텐츠).
- "패킷 하나 보내고 프레임 하나 받는다"는 DEC-001의 오해와 함께 나타나는 경우가 많다.

**권장**:
```rust
fn decode_and_collect(ctx: *mut AVCodecContext, pkt: *mut AVPacket, out: &mut Vec<Frame>) {
    unsafe {
        avcodec_send_packet(ctx, pkt);
        loop {
            let raw = av_frame_alloc();
            let ret = avcodec_receive_frame(ctx, raw);
            if ret < 0 {
                av_frame_free(&mut (raw as *mut _));
                break;
            }
            // 디코더가 재정렬을 마친 뒤 프레임 자체에 채워준 pts를 사용한다.
            // pkt->pts를 별도로 들고 다니지 않는다.
            out.push(Frame::from_raw(raw));
        }
    }
}
```
- 프레임의 타임스탬프는 항상 `AVFrame.pts`(디코더가 재정렬을 반영해 채운 값)에서 읽는다. 마지막으로 보낸 패킷의 pts를 별도 변수에 캐싱해 프레임에 재사용하지 않는다.
- reorder가 없는 코덱이라도 습관적으로 프레임 자체의 필드를 신뢰하는 코드 경로를 하나로 통일해 둔다.

**탐지 방법**:
- Semantic: `pkt.pts`/`pkt->pts` 값이 `send_packet` 호출 이후 `receive_frame`으로 얻은 프레임의 pts 필드에 대입되는 데이터플로 탐지.
- Runtime: 알려진 GOP 구조(B-frame 포함)의 테스트 벡터에서 디코드된 프레임의 pts 순서가 단조 증가하는지 검증.

**예외**:
- 컨테이너가 pts를 아예 제공하지 않고(pts == AV_NOPTS_VALUE) dts만 있는 극히 예외적인 raw 스트림에서는 별도의 fallback 로직(DEC-006 참고)이 필요하며, 이 경우도 "패킷 pts를 그대로 복사"가 아니라 명시적 fallback 규칙으로 처리해야 한다.

**Bitvue 판정**: Suspected — `FfmpegDecoder::ffmpeg_frame_to_decoded`(crates/bitvue-decode/src/ffmpeg.rs:246)는 `frame.timestamp()`(프레임 자체 필드)를 우선 사용하고 값이 없을 때만 `self.timestamp`(마지막으로 보낸 패킷의 캐시된 ts)로 폴백하므로 "패킷 pts를 무조건 복사"하는 나쁜 예와는 다르지만, 폴백 경로에서는 여전히 재정렬된 프레임에 stale한 패킷 타임스탬프가 붙을 수 있다. AV1(dav1d) 경로는 `picture.timestamp()`(decoder.rs:516)만 사용해 이 문제가 없어 보인다.

---

### DEC-006: best_effort_timestamp 무시
**분류**: DEC · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn frame_display_time(frame: &Frame, time_base: AVRational) -> f64 {
    // frame.pts가 AV_NOPTS_VALUE(컨테이너가 pts를 안 준 경우)일 수 있음을 고려하지 않음
    frame.pts as f64 * time_base.num as f64 / time_base.den as f64
}
```

**문제**:
- 일부 컨테이너/스트림 조합(특히 raw Annex B, 일부 legacy MKV/mux)에서는 `AVFrame.pts`가 `AV_NOPTS_VALUE`로 비어 있고 dts만 유효한 경우가 있다.
- FFmpeg는 이런 경우를 보정해 `best_effort_timestamp`(dts 기반 추정치 포함)를 제공하는데, 이를 쓰지 않고 raw `pts`만 참조하면 그 프레임의 표시 시각이 통째로 결측되거나 0/음수로 계산된다.
- 타임라인 UI에서 프레임이 원점에 몰리거나 순서가 깨지는 형태로 나타난다.

**발생 조건**:
- 컨테이너 메타데이터가 불완전한 파일(특히 사용자가 외부에서 가져온 손상/재mux된 테스트 파일).
- raw elementary stream을 컨테이너 없이 직접 디코드할 때(pts 정보 자체가 없어 dts로부터 추정해야 함).

**권장**:
```rust
fn frame_display_time(frame: &AVFrame, time_base: AVRational) -> Option<f64> {
    let ts = frame.best_effort_timestamp; // pts가 없으면 dts 기반 추정치로 폴백
    if ts == AV_NOPTS_VALUE {
        return None; // 명시적으로 "타임스탬프 없음"을 표현하고 호출부가 처리
    }
    Some(ts as f64 * time_base.num as f64 / time_base.den as f64)
}
```
- 표시 시각 계산에는 `pts` 대신 `best_effort_timestamp`를 사용한다.
- 그래도 `AV_NOPTS_VALUE`인 경우를 `Option`으로 명시해 호출부가 "이 프레임은 시각 정보가 없다"를 인지하고 처리하게 한다(예: 이전 프레임 + 평균 프레임 duration으로 보간).

**탐지 방법**:
- Static: `frame.pts`/`frame->pts`를 직접 참조하면서 `best_effort_timestamp`나 `AV_NOPTS_VALUE` 체크가 코드베이스 어디에도 없는 경우 grep.
- Manual: pts가 결측된 테스트 파일을 하나 확보해 수동 회귀.

**예외**:
- dav1d처럼 애초에 컨테이너 pts를 다루지 않고 호출자가 직접 프레임-타임스탬프 매핑을 관리하는 raw 디코더 API를 쓸 때는 `best_effort_timestamp` 개념 자체가 없으므로, 호출자가 own 매핑 테이블(패킷→pts)을 프레임 방출 순서에 맞춰 재정렬하는 자체 로직으로 대체해야 한다.

**Bitvue 판정**: Suspected — `ffmpeg-next`의 `Frame::timestamp()`가 내부적으로 `best_effort_timestamp`를 쓰는지 raw `pts`를 쓰는지 로컬 소스로 확인할 수 없었다(크레이트 소스가 오프라인 캐시에 없음). 이 코드베이스 자체에는 `best_effort_timestamp`/`AV_NOPTS_VALUE`를 명시적으로 다루는 코드가 없다. dav1d 경로는 카탈로그가 명시한 예외(컨테이너 pts 개념 자체가 없음)에 해당해 그쪽은 N/A에 가깝다.

---

### DEC-007: hw frame과 sw frame lifetime 혼동
**분류**: DEC · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
fn get_frame_pixels(frame: &AVFrame) -> &[u8] {
    // hwaccel 디코드 시 frame->data[]는 GPU 메모리(VA-API surface, CUDA 등)를 가리키는
    // 불투명 핸들일 뿐 CPU에서 바로 읽을 수 있는 포인터가 아님
    unsafe { std::slice::from_raw_parts(frame.data[0], (frame.width * frame.height) as usize) }
}
```

**문제**:
- `hw_frames_ctx`가 설정된 디코더의 출력 프레임은 `format`이 hwaccel pixel format(예: `AV_PIX_FMT_VAAPI`, `AV_PIX_FMT_CUDA`)이며 `data[]`는 GPU 메모리 핸들/서페이스 ID이지 CPU에서 직접 dereference 가능한 픽셀 버퍼가 아니다.
- 이를 sw 프레임처럼 취급해 CPU에서 바로 읽으면 세그폴트, 쓰레기 데이터, 혹은 (운 나쁘게) 우연히 유효해 보이는 다른 GPU 자원의 메모리를 읽는 결과가 나온다.
- 반대로 hw 프레임의 lifetime을 sw 프레임처럼 오래 들고 있으면(예: 재생 버퍼에 수십 개 hw 프레임 캐시) 제한된 GPU 디코드 서페이스 풀이 고갈되어 디코더가 멈춘다.

**발생 조건**:
- 하드웨어 가속 디코드 경로(VAAPI/VideoToolbox/CUDA/D3D11VA)를 켰을 때, sw 디코드 경로와 동일한 프레임 처리 코드를 공유하도록 잘못 설계한 경우.
- 분석기가 프레임 픽셀에 직접 접근해야 하는 기능(히스토그램, 히트맵, 픽셀 인스펙터)에서 hw 프레임을 sw로 전송(`av_hwframe_transfer_data`)하는 과정을 빼먹은 경우.

**권장**:
```rust
unsafe fn ensure_sw_frame(hw_frame: *mut AVFrame) -> Result<*mut AVFrame, DecodeError> {
    if (*hw_frame).hw_frames_ctx.is_null() {
        return Ok(hw_frame); // 이미 sw 프레임
    }
    let sw_frame = av_frame_alloc();
    let ret = av_hwframe_transfer_data(sw_frame, hw_frame, 0);
    if ret < 0 {
        av_frame_free(&mut (sw_frame as *mut _));
        return Err(DecodeError::from_averror(ret));
    }
    // 색공간/범위 등 메타데이터도 함께 복사되어야 함(DEC-019 참고)
    av_frame_copy_props(sw_frame, hw_frame);
    Ok(sw_frame)
}
```
- hw 프레임 픽셀에 접근해야 하는 모든 경로 앞에서 `format`을 확인하고, hwaccel 포맷이면 `av_hwframe_transfer_data`로 명시적으로 sw 메모리에 복사한 뒤 그 사본을 사용한다.
- hw 프레임 자체는 전송이 끝나는 즉시(또는 GPU 디코드 서페이스 풀 압박을 피할 수 있는 최단 시간 내) 해제한다. sw 사본만 필요한 만큼 오래 보관한다.

**탐지 방법**:
- Runtime: hwaccel 디코드 경로를 켠 상태로 픽셀 접근 기능(히스토그램 등)을 실행하는 통합 테스트에서 크래시/쓰레기 값 여부 확인.
- Static: `frame.data[0]`를 직접 슬라이스로 변환하는 코드 경로에서 `hw_frames_ctx`/`format` 분기 존재 여부 검사.

**예외**:
- 순수 sw 디코드 전용으로 빌드/설정된 경로(hwaccel 비활성)에서는 이 문제가 발생하지 않지만, 향후 hwaccel을 추가할 가능성이 있다면 처음부터 `format` 분기를 두는 편이 안전하다.

**Bitvue 판정**: N/A — 코드베이스에 hwaccel 경로 자체가 없다. `FfmpegDecoder::capabilities()`가 `hw_accel: false`를 하드코딩(crates/bitvue-decode/src/ffmpeg.rs:342)하고, VAAPI/VideoToolbox/CUDA/`hw_frames_ctx` 관련 참조가 bitvue-decode 어디에도 없다(grep 결과 0건).

---

### DEC-008: frame reference 해제를 늦게 해 decoder pool 고갈
**분류**: DEC · **심각도**: High · **탐지**: Runtime

**나쁜 예**:
```rust
struct FrameCache {
    frames: Vec<Frame>, // 디코드된 프레임을 무제한 누적 보관 (레퍼런스 카운트 유지)
}

impl FrameCache {
    fn on_decoded(&mut self, frame: Frame) {
        self.frames.push(frame); // 언제 drop되는지 아무도 관리하지 않음
    }
}
```

**문제**:
- FFmpeg의 `get_buffer2`/내부 프레임 풀은 유한하다(특히 hwaccel 서페이스 풀, 저지연 인코더의 reference frame pool). 디코드된 `AVFrame`은 refcount 기반이며, 애플리케이션이 참조를 계속 들고 있으면 그 버퍼는 풀로 반환되지 않는다.
- 분석기가 "나중에 다시 볼 수도 있으니" 모든 프레임을 무기한 캐시하면, 디코더가 다음 프레임을 디코드하기 위한 여유 버퍼를 확보하지 못해 `receive_frame`이 영구적으로 `EAGAIN`을 반환하거나 hwaccel 서페이스 할당이 실패한다.
- 증상이 "디코드가 멈춘다"로 나타나 원인이 프레임 캐시 누수라는 것을 진단하기 어렵다.

**발생 조건**:
- 긴 영상을 전부 메모리에 올려 프레임 단위 랜덤 접근을 지원하려는 설계에서, 상한 없는 캐시를 둔 경우.
- hwaccel 서페이스 풀 크기가 작게 설정된 환경(임베디드, 일부 VAAPI 드라이버)일수록 더 적은 프레임 만에 고갈된다.

**권장**:
```rust
struct BoundedFrameCache {
    frames: std::collections::VecDeque<Frame>,
    capacity: usize,
}

impl BoundedFrameCache {
    fn on_decoded(&mut self, frame: Frame) {
        self.frames.push_back(frame);
        while self.frames.len() > self.capacity {
            self.frames.pop_front(); // 가장 오래된 참조부터 drop -> 디코더 풀에 반환
        }
    }
}
```
- 프레임 캐시는 명시적 상한(개수 또는 바이트)을 두고, 상한 초과 시 가장 오래된 프레임부터 `drop`하여 내부 refcount를 즉시 0으로 만든다.
- 화면에 필요한 것은 픽셀 데이터뿐이라면, `AVFrame` 레퍼런스 대신 필요한 시점에 sw 버퍼로 복사한 경량 스냅샷(예: RGBA 썸네일)만 장기 보관한다.

**탐지 방법**:
- Runtime: 긴 스트림(수천 프레임)을 전 구간 디코드하는 부하 테스트에서 디코드 처리량이 시간이 지날수록 저하되거나 특정 지점에서 멈추는지 관찰.
- Structural: 디코드 결과 프레임을 담는 컬렉션에 상한/축출(eviction) 로직이 있는지 코드 리뷰 체크리스트로 확인.

**예외**:
- 짧은 클립(수 초~수십 초) 전용 분석 도구처럼 전체 프레임 수가 애초에 풀 크기보다 훨씬 작음이 보장된 경우는 무제한 캐시가 실용적으로 문제되지 않을 수 있다. 다만 파일 크기 제한이 바뀌면 재발하므로 상한을 두는 편이 안전하다.

**Bitvue 판정**: N/A/absent — 발견된 프레임 캐시는 모두 명시적 상한 + LRU 축출을 갖춘다: `FfmpegDecoder::frame_buffer`(`MAX_FRAME_BUFFER_SIZE=16`, ffmpeg.rs:17,126-131), `ThumbnailService`(`MAX_CACHE_SIZE=200`, thumbnail_service.rs:17,154-159), `DecodeService`의 rgb/yuv 캐시(바이트 예산 기반 LRU, decode_service.rs:320-322 등). 무제한 캐시 패턴은 확인되지 않았다.

---

### DEC-009: seek 후 decoder flush 누락
**분류**: DEC · **심각도**: Critical · **탐지**: Structural

**나쁜 예**:
```rust
fn seek_and_decode(ctx: *mut AVCodecContext, fmt: *mut AVFormatContext, target_pts: i64) -> Vec<Frame> {
    unsafe {
        av_seek_frame(fmt, -1, target_pts, AVSEEK_FLAG_BACKWARD);
        // avcodec_flush_buffers 없이 바로 새 위치의 패킷을 기존 디코더에 계속 투입
    }
    let mut frames = Vec::new();
    decode_from_current_position(ctx, fmt, &mut frames);
    frames
}
```

**문제**:
- `av_seek_frame`은 컨테이너 레벨에서 읽기 위치만 옮길 뿐, 디코더 내부 상태(참조 프레임 버퍼, reorder 큐, 엔트로피 상태)는 그대로 이전 위치의 것을 들고 있다.
- flush 없이 새 위치의 패킷을 넣으면 디코더가 존재하지 않는(또는 시간적으로 멀리 떨어진) 참조 프레임을 가리키는 모션 벡터/예측을 그대로 사용해 블록 깨짐, 색 번짐, 크래시가 발생할 수 있다.
- 이런 손상은 seek 직후 몇 프레임에서만 나타나다가 다음 keyframe에서 "저절로" 사라지므로, 원인 파악 없이 "가끔 깨지는 현상"으로 방치되기 쉽다.

**발생 조건**:
- 타임라인 스크러빙, 랜덤 접근 프레임 뷰어, "N번째 프레임으로 이동" 기능처럼 seek을 반복하는 모든 UI 경로.
- 동일 `AVCodecContext`를 재사용하며 여러 번 seek할 때마다 매번 재발한다(한 번 고쳐도 다른 seek 경로에 빠져 있으면 재현).

**권장**:
```rust
fn seek_and_decode(ctx: *mut AVCodecContext, fmt: *mut AVFormatContext, target_pts: i64) -> Vec<Frame> {
    unsafe {
        av_seek_frame(fmt, -1, target_pts, AVSEEK_FLAG_BACKWARD);
        avcodec_flush_buffers(ctx); // 디코더 내부 상태(참조 프레임, reorder 큐)를 리셋
    }
    let mut frames = Vec::new();
    decode_from_current_position(ctx, fmt, &mut frames);
    frames
}
```
- 컨테이너 seek 직후 반드시 `avcodec_flush_buffers`(또는 dav1d의 컨텍스트 재초기화/flush 대응 API)를 호출해 디코더를 깨끗한 상태로 되돌린다.
- seek 진입점이 여러 곳(스크러빙, 프레임 점프, 구간 반복재생)이라면 "seek → flush"를 하나의 헬퍼 함수로 강제해 누락 가능성을 구조적으로 없앤다.

**탐지 방법**:
- Structural: `av_seek_frame`/`avformat_seek_file` 호출 이후 같은 함수 또는 호출 경로 내에 `avcodec_flush_buffers` 호출이 없는 패턴을 제어흐름으로 탐지.
- Runtime: 스크러빙을 반복하는 통합 테스트에서 seek 직후 프레임의 시각적 손상(참조 프레임 검증 실패, PSNR 급락) 여부 확인.

**예외**:
- keyframe-only 스트림(all-intra)에서 seek이 항상 keyframe 경계와 정확히 일치한다면 실질적 영향은 적을 수 있으나, 디코더 내부 reorder 큐 상태는 여전히 리셋해주는 편이 안전하고 비용도 낮다.

**Bitvue 판정**: N/A — 이 코드베이스는 열린 `AVFormatContext`+디코더 세션을 계속 들고 있다가 seek하는 구조 자체가 없다. "N번째 프레임 요청"마다 매번 새 디코더를 만들어 처음부터(또는 DEC-011처럼 해당 샘플 단독으로) 다시 디코드하므로 `av_seek_frame` 이후 flush를 빼먹는 경로가 존재하지 않는다. 유일한 실제 `seek_to_frame`(crates/bitvue-decode/src/yuv_loader.rs:405)은 압축 스트림이 아닌 raw YUV/Y4M 전용이라 디코더 상태 자체가 없다.

---

### DEC-010: seek 지점부터 바로 target frame이라고 가정
**분류**: DEC · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn get_frame_at(fmt: *mut AVFormatContext, ctx: *mut AVCodecContext, target_pts: i64) -> Frame {
    unsafe {
        av_seek_frame(fmt, -1, target_pts, AVSEEK_FLAG_BACKWARD);
        avcodec_flush_buffers(ctx);
    }
    // seek 후 처음 디코드되는 프레임을 곧바로 target이라고 간주
    decode_one_frame(ctx, fmt).expect("no frame")
}
```

**문제**:
- `AVSEEK_FLAG_BACKWARD` seek은 target_pts "이전 또는 같은" 가장 가까운 키프레임(sync point)으로 이동할 뿐, target_pts 프레임 자체로 이동하지 않는다.
- seek 직후 첫 디코드 프레임은 대개 키프레임이며 target보다 훨씬 앞선 프레임이다. 이를 target으로 잘못 반환하면 "프레임 100으로 이동"했는데 실제로는 프레임 85(직전 keyframe)가 표시되는 식의 오차가 발생한다.
- GOP 길이가 길수록(키프레임 간격이 클수록) 오차가 커지며, 사용자에게는 "정밀 프레임 이동이 안 먹힌다"는 체감으로 나타난다.

**발생 조건**:
- 키프레임이 아닌 임의 프레임으로 이동하는 모든 기능(프레임 번호 입력, 타임코드 이동, 특정 PTS로 이동).
- 긴 GOP(예: 250프레임)를 사용하는 스트림에서 오차가 크게 체감된다.

**권장**:
```rust
fn get_frame_at(fmt: *mut AVFormatContext, ctx: *mut AVCodecContext, target_pts: i64, time_base: AVRational) -> Frame {
    unsafe {
        av_seek_frame(fmt, -1, target_pts, AVSEEK_FLAG_BACKWARD);
        avcodec_flush_buffers(ctx);
    }
    // 키프레임부터 target_pts까지 디코드하며 "버리는" 프레임들을 명시적으로 처리
    loop {
        let frame = decode_one_frame(ctx, fmt).expect("no frame before target");
        let ts = frame_best_effort_pts(&frame);
        if ts >= target_pts {
            return frame; // target에 도달(또는 근접)한 프레임만 반환
        }
        // 그 이전 프레임들은 디코드는 하되 표시하지 않고 버림
    }
}
```
- seek 직후부터 target까지는 "디코드는 하되 표시하지 않는" 워밍업 구간으로 명시적으로 설계한다.
- 정확한 프레임 매칭이 필요한 UI는 "keyframe으로 점프"와 "정확한 프레임으로 점프"를 별도 동작으로 구분해 사용자에게도 어떤 것이 일어나는지 명확히 한다.

**탐지 방법**:
- Semantic: seek 함수 반환값을 곧바로 "target frame"으로 이름 붙여 사용하는 코드에서 실제 pts 비교 로직 존재 여부 검사.
- Manual/Runtime: 긴 GOP 테스트 벡터에서 임의 프레임 번호로 이동 후 반환된 프레임의 실제 pts가 요청한 target과 일치하는지 확인.

**예외**:
- "가장 가까운 keyframe으로 빠르게 미리보기"가 기능 자체의 요구사항(예: 타임라인 썸네일 스크러빙에서 정밀도보다 반응성이 중요한 경우)이라면 이 동작이 의도된 것일 수 있다. 이 경우 UI에 "근사치"임을 명시하는 것으로 충분하다.

**Bitvue 판정**: N/A — DEC-009와 동일한 이유(라이브 seek 가능한 디코더 세션이 없음)로 "seek 지점 = target frame" 가정 자체가 성립할 여지가 없다. 다만 이 아키텍처는 그 대신 DEC-011에서 지적하는, seek보다 더 심각한 문제(단일 프레임만 디코드)를 갖고 있다.

---

### DEC-011: keyframe 이전 dependency 무시
**분류**: DEC · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn decode_from_index(fmt: *mut AVFormatContext, ctx: *mut AVCodecContext, packet_index: usize, packets: &[Packet]) -> Vec<Frame> {
    let mut frames = Vec::new();
    // 컨테이너 인덱스가 가리키는 위치부터 곧바로 디코드 시작
    // 그 패킷이 실제로 IDR/keyframe인지 확인하지 않음
    for pkt in &packets[packet_index..] {
        unsafe { decode_packet_loop(ctx, pkt.as_raw(), &mut frames) };
    }
    frames
}
```

**문제**:
- 컨테이너의 프레임 인덱스나 사용자가 지정한 패킷 오프셋이 항상 keyframe(IDR/sync point)이라는 보장은 없다. 손상된 인덱스, 근사 seek 테이블, 수동으로 계산한 오프셋에서는 non-keyframe 패킷부터 디코드를 시작할 수 있다.
- 디코더가 non-keyframe부터 시작하면 참조할 이전 프레임이 없어(reference frame이 초기화되지 않은 상태) 예측 오차가 그대로 노출되어 녹색/회색 블록, 심한 블로킹 아티팩트, 또는 즉시 디코드 에러가 발생한다.
- HEVC/AV1처럼 여러 종류의 랜덤 액세스 포인트(IDR, CRA, BLA / key frame, intra-only frame)를 구분해야 하는 코덱에서는 "keyframe처럼 보이지만 실제로는 leading picture에 의존하는" 미묘한 케이스(HEVC CRA의 RASL 프레임)도 있다.

**발생 조건**:
- 컨테이너 인덱스가 부정확하거나 재mux 과정에서 sync point 플래그가 유실된 파일.
- 사용자가 임의 바이트/패킷 오프셋에서 분석을 시작하도록 허용하는 "raw stream 부분 로드" 기능.
- 세그먼트 스트리밍(HLS/DASH) 콘텐츠에서 세그먼트 경계가 항상 IDR과 일치한다고 가정했지만 실제로는 아닌 경우.

**권장**:
```rust
fn find_valid_start(packets: &[Packet], hint_index: usize) -> usize {
    // hint 위치부터 뒤로 스캔하며 실제 keyframe/IDR 플래그를 가진 패킷을 찾는다
    (0..=hint_index)
        .rev()
        .find(|&i| packets[i].is_key_frame())
        .unwrap_or(0)
}

fn decode_from_index(fmt: *mut AVFormatContext, ctx: *mut AVCodecContext, hint_index: usize, packets: &[Packet]) -> Vec<Frame> {
    let start = find_valid_start(packets, hint_index);
    let mut frames = Vec::new();
    for pkt in &packets[start..] {
        unsafe { decode_packet_loop(ctx, pkt.as_raw(), &mut frames) };
    }
    frames
}
```
- 디코드를 시작할 패킷은 컨테이너/사용자 힌트를 그대로 믿지 않고 실제 `AV_PKT_FLAG_KEY` 플래그(또는 코덱별 NAL/OBU 타입 파싱 결과)로 검증한다.
- HEVC CRA + RASL처럼 "keyframe이지만 일부 후행 프레임이 그 이전 GOP에 의존"하는 케이스는 코덱별 랜덤 액세스 규칙을 별도로 문서화하고 처리한다.

**탐지 방법**:
- Semantic: 디코드 시작 인덱스를 컨테이너 인덱스/사용자 입력에서 그대로 가져와 keyframe 플래그 검증 없이 사용하는 코드 경로 추적.
- Manual: 손상된 인덱스를 가진 실제 파일 또는 인위적으로 잘라낸 non-keyframe 시작 스트림으로 수동 검증.

**예외**:
- 항상 전체 파일을 처음(첫 keyframe)부터 순차 디코드하고 임의 오프셋 시작을 지원하지 않는 단순 배치 분석 도구라면 이 문제는 구조적으로 발생하지 않는다.

**Bitvue 판정**: N/A — (재검증, 이전 판정은 stale) 근거였던 `decode_ivf_frame_generic`/`decode_container_frame_yuv_with_samples`(src-tauri/src/commands/frame.rs)는 `src-tauri` 삭제(`e7194cc`)로 더 이상 존재하지 않는다. 현재 `get_decoded_frame_yuv`(crates/bitvue-sidecar/src/decode_bridge.rs:40-77)는 정확히 이 안티패턴을 피하도록 설계돼 있다 — 모듈 상단 doc comment가 "re-decodes from the start of the stream up to the target frame on every call"이라고 명시하고, 실제로 `for f in &frames`가 스트림 맨 앞(프레임 0)부터 target까지 전부 디코더에 순서대로 투입한다(52-59행). 참조 프레임 체인이 항상 완전하므로 non-keyframe 단독 디코드 문제가 발생하지 않는다(대신 매 요청마다 O(frame_index) 재디코드하는 성능 트레이드오프가 있으나 이는 별개 카테고리인 PERF/RPERF 소관).

---

### DEC-012: codec context를 여러 작업에서 공유
**분류**: DEC · **심각도**: Critical · **탐지**: Runtime

**나쁜 예**:
```rust
struct AppState {
    decoder_ctx: *mut AVCodecContext, // 앱 전체에서 단 하나만 생성
}

// UI 스레드: 재생 중 현재 프레임 디코드
fn playback_tick(state: &AppState, pkt: &Packet) -> Frame { /* decoder_ctx 사용 */ }

// 백그라운드 스레드: 썸네일 생성을 위해 임의 위치 seek+디코드
fn generate_thumbnail(state: &AppState, pts: i64) -> Frame {
    // 재생에 쓰이는 것과 동일한 decoder_ctx를 그대로 seek/flush/디코드
}
```

**문제**:
- `AVCodecContext`는 내부에 참조 프레임, reorder 버퍼, 엔트로피 디코더 상태 등 mutable 상태를 가진 단일 스트림 디코드 세션이다. 두 개의 독립적인 논리적 작업(재생 위치 디코드 vs 썸네일용 임의 위치 디코드)이 같은 컨텍스트를 공유하면 서로의 seek/flush가 상대방의 디코드 상태를 파괴한다.
- 스레드 안전성이 보장되지 않는 API(`avcodec_send_packet`/`receive_frame`은 동일 컨텍스트에 대해 스레드 세이프하지 않음)를 잠금 없이 여러 스레드에서 호출하면 데이터 레이스로 인해 크래시 또는 조용한 프레임 손상이 발생한다.
- 잠금을 걸어 직렬화하더라도, "재생 중인데 썸네일 요청이 끼어들어 seek해버려서 재생이 끊기는" 논리적 오작동은 여전히 남는다.

**발생 조건**:
- 재생, 썸네일 생성, 프레임 단위 메트릭 계산, 프레임 익스포트 등 서로 다른 목적의 디코드 요청이 동시에 들어올 수 있는 멀티스레드/비동기 아키텍처.
- 성능을 이유로 "디코더 초기화 비용을 아끼자"며 컨텍스트를 재사용하도록 설계했을 때.

**권장**:
```rust
struct DecoderPool {
    // 목적별로 독립적인 디코더 컨텍스트를 분리 소유
    playback: Mutex<DecoderSession>,
    thumbnail: Mutex<DecoderSession>,
}

impl DecoderPool {
    fn playback_decode(&self, pkt: &Packet) -> Frame {
        let mut session = self.playback.lock().unwrap();
        session.decode(pkt)
    }

    fn thumbnail_decode(&self, pts: i64) -> Frame {
        let mut session = self.thumbnail.lock().unwrap();
        session.seek_and_decode(pts) // 재생용 세션과 완전히 독립된 상태
    }
}
```
- 목적이 다른 디코드 작업(순차 재생 vs 임의 seek 기반 썸네일/익스포트)은 각각 독립된 `AVCodecContext` 인스턴스를 소유하게 한다. 초기화 비용은 캐시된 파라미터(`extradata` 등)로 줄이되 상태 공유는 하지 않는다.
- 정말 컨텍스트를 공유해야 한다면 하나의 논리적 소유자(단일 스레드 또는 명시적 락)만 접근하도록 강제하고, "동시에 서로 다른 위치를 디코드하지 않는다"는 불변조건을 문서화·테스트한다.

**탐지 방법**:
- Runtime: 재생과 썸네일 생성을 동시에 트리거하는 동시성 스트레스 테스트(ThreadSanitizer/loom류)로 데이터 레이스 또는 재생 프레임 손상 여부 확인.
- Structural: 동일 `AVCodecContext`/`Dav1dContext` 핸들이 두 개 이상의 독립적 호출 경로(재생 루프, 썸네일 함수 등)에서 참조되는지 소유권 그래프로 탐지.

**예외**:
- 완전히 단일 스레드·단일 목적(예: 배치 CLI 도구가 파일 하나를 순차 디코드만 하는 경우)에서는 공유 자체가 문제되지 않는다.

**Bitvue 판정**: N/A/absent — 공유되는 영속 디코더 인스턴스를 찾지 못했다. frame.rs/quality.rs/thumbnail_service.rs의 모든 커맨드 핸들러가 호출마다 로컬 `Av1Decoder`/`FfmpegDecoder`를 새로 만들고 함수 종료 시 drop한다(`DecodeService`에도 디코더 필드 없음, decode_service.rs:115 구조체 정의 확인). 따라서 재생용/썸네일용 작업이 같은 컨텍스트를 두고 경합할 여지가 없다(다만 DEC-011처럼 매번 새로 만드는 대가로 다른 문제가 생긴다).

---

### DEC-013: swscale context를 매 프레임 생성
**분류**: DEC · **심각도**: Medium · **탐지**: Static

**나쁜 예**:
```rust
fn convert_to_rgb(frame: &AVFrame) -> Vec<u8> {
    unsafe {
        // 매 프레임마다 새로 컨텍스트를 만들고 버림
        let sws = sws_getContext(
            frame.width, frame.height, frame.format,
            frame.width, frame.height, AV_PIX_FMT_RGBA,
            SWS_BILINEAR, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(),
        );
        let mut rgb = vec![0u8; (frame.width * frame.height * 4) as usize];
        // ... sws_scale 호출 ...
        sws_freeContext(sws);
        rgb
    }
}
```

**문제**:
- `sws_getContext`는 변환 테이블(색공간 행렬, 필터 계수 등) 계산을 포함한 상대적으로 무거운 초기화이며, 입력/출력 포맷·크기가 프레임마다 바뀌지 않는 한 매번 다시 만들 이유가 없다.
- 초당 수십~수백 프레임을 처리하는 재생/분석 루프에서 이 오버헤드가 누적되면 눈에 띄는 CPU 낭비와 프레임 드랍으로 이어진다.
- 힙 할당/해제도 매 프레임 반복되어 메모리 할당자 압박과 캐시 지역성 저하를 유발한다.

**발생 조건**:
- 프레임 단위 색공간/픽셀포맷 변환(YUV→RGB 등)이 재생 루프, 실시간 미리보기, 프레임별 메트릭 계산 파이프라인 어디에든 있으면 발생.
- 입출력 크기·포맷이 스트림 전체에서 고정적인 일반적인 경우 특히 낭비가 크다(변할 이유가 없는데 매번 재생성).

**권장**:
```rust
struct RgbConverter {
    ctx: *mut SwsContext,
    src: (i32, i32, i32), // width, height, pix_fmt
}

impl RgbConverter {
    unsafe fn get_or_create(&mut self, frame: &AVFrame) -> *mut SwsContext {
        let key = (frame.width, frame.height, frame.format);
        if key != self.src || self.ctx.is_null() {
            if !self.ctx.is_null() {
                sws_freeContext(self.ctx);
            }
            // sws_getCachedContext로 기존 컨텍스트를 조건이 맞으면 그대로 재사용
            self.ctx = sws_getCachedContext(
                std::ptr::null_mut(),
                key.0, key.1, key.2,
                key.0, key.1, AV_PIX_FMT_RGBA,
                SWS_BILINEAR, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null(),
            );
            self.src = key;
        }
        self.ctx
    }
}
```
- `SwsContext`를 스트림/세션 단위로 캐시하고, 입출력 포맷·크기가 바뀔 때만 재생성한다(`sws_getCachedContext`로 조건부 재사용을 아예 라이브러리에 위임할 수도 있다).
- 변환기를 담당 객체(예: `RgbConverter`)로 캡슐화해 프레임 루프 코드가 캐싱 세부사항을 신경 쓰지 않게 한다.

**탐지 방법**:
- Static: 프레임 처리 함수(매 프레임 호출되는 함수) 본문 안에 `sws_getContext` 호출이 있는지 grep.
- Runtime: 프로파일러로 `sws_getContext`/`sws_freeContext`가 프레임 수만큼 호출되는지 CPU 타임라인에서 확인.

**예외**:
- 프레임마다 실제로 해상도나 픽셀 포맷이 바뀌는 스트림(적응형 해상도, mid-stream 포맷 변경)이라면 재생성이 불가피하다. 이 경우에도 "바뀔 때만" 재생성하도록 캐시 키 비교는 유지해야 한다.

**Bitvue 판정**: N/A/absent — `FfmpegDecoder`의 `SafeScaler`는 `src_format`/`width`/`height`가 바뀔 때만 재생성하도록 캐시되어 있다(crates/bitvue-decode/src/ffmpeg.rs:178-198, `needs_new_scaler` 체크). 나쁜 예가 아니라 권장 패턴과 동일하게 구현되어 있다.

---

### DEC-014: pixel format negotiation 결과를 고정값으로 가정
**분류**: DEC · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn read_luma_plane(frame: &AVFrame) -> &[u8] {
    // 디코더 출력이 항상 8비트 YUV420P라고 하드코딩
    let size = (frame.width * frame.height) as usize;
    unsafe { std::slice::from_raw_parts(frame.data[0], size) }
}
```

**문제**:
- 디코더의 실제 출력 픽셀 포맷은 스트림의 bit depth(8/10/12비트), 크로마 서브샘플링(4:2:0/4:2:2/4:4:4), hwaccel 사용 여부에 따라 달라진다. `get_format` 콜백(FFmpeg) 또는 동등한 negotiation 결과를 무시하고 8비트 `YUV420P`를 고정 가정하면, 10비트 HDR 콘텐츠(`YUV420P10LE`, 리틀엔디안 16비트 컨테이너에 10비트 값)에서 plane stride/바이트당 샘플 크기 계산이 전부 틀어진다.
- `frame.width * frame.height`로 luma plane 크기를 계산하는 것 자체가 10비트에서는 절반만 읽거나(바이트 수 착각) linesize(stride)와 실제 유효 너비가 다른 경우(정렬 패딩)를 무시하는 이중의 오류다.
- 결과 이미지가 밴딩/색 어긋남/절반만 보이는 형태로 깨지며, 원인이 "포맷 협상 무시"라는 것을 UI 증상만으로는 알기 어렵다.

**발생 조건**:
- HDR10/Dolby Vision 등 10비트 이상 콘텐츠, 4:2:2/4:4:4 크로마를 쓰는 프로 콘텐츠, hwaccel 협상으로 출력 포맷이 sw 포맷과 달라지는 경우.
- 처음 테스트할 때 8비트 4:2:0 샘플 파일만 사용해 하드코딩이 통과해버리고, 나중에 실사용 HDR 파일에서 발현.

**권장**:
```rust
fn read_luma_plane(frame: &AVFrame) -> PlaneView<'_> {
    let bytes_per_sample = match frame.format {
        AV_PIX_FMT_YUV420P | AV_PIX_FMT_YUV422P | AV_PIX_FMT_YUV444P => 1,
        AV_PIX_FMT_YUV420P10LE | AV_PIX_FMT_YUV422P10LE | AV_PIX_FMT_YUV444P10LE => 2,
        other => panic!("처리되지 않은 pixel format: {other:?}"), // 명시적으로 실패시켜 누락을 드러냄
    };
    PlaneView {
        data: unsafe { std::slice::from_raw_parts(frame.data[0], (frame.linesize[0] * frame.height) as usize) },
        stride: frame.linesize[0] as usize, // 실제 너비가 아니라 stride 기준으로 접근
        width: frame.width as usize,
        bytes_per_sample,
    }
}
```
- 디코더가 `get_format` 콜백(또는 dav1d의 picture 구조체가 보고하는 `bpc`/`layout`)으로 알려주는 실제 출력 포맷을 매 세션(또는 매 프레임, 포맷 변경 스트림이라면)마다 조회해 그 값을 기준으로 plane 크기/stride/bit depth를 계산한다.
- plane 접근은 항상 `linesize`(stride)를 기준으로 하고, `width`는 유효 픽셀 수 계산에만 쓴다.

**탐지 방법**:
- Semantic: pixel format을 상수로 하드코딩하고 `frame.format`/`get_format` 콜백 결과와 비교하지 않는 plane 접근 코드 탐지.
- Runtime: 10비트/4:2:2/4:4:4 테스트 벡터로 픽셀 접근 기능을 실행해 크래시/이미지 깨짐 여부 회귀 테스트.

**예외**:
- 애플리케이션이 의도적으로 특정 포맷만 지원 범위로 못박고(예: "8비트 4:2:0만 지원") 그 외 포맷은 디코드 파이프라인 진입 전에 명시적으로 거부하는 설계라면, 이후 코드에서 고정 가정을 쓰는 것이 오히려 의도된 단순화일 수 있다. 이 경우 거부 로직이 실제로 모든 진입점에서 강제되는지가 핵심이다.

**Bitvue 판정**: Confirmed(단, 현재 죽은 코드 한정) — `FfmpegDecoder::ffmpeg_frame_to_decoded`는 `bit_depth: 8, // FFmpeg typically outputs 8-bit`를 하드코딩한다(crates/bitvue-decode/src/ffmpeg.rs:258,315). 게다가 `Pixel::YUV420P`가 아닌 모든 출력(10/12비트, 4:2:2/4:4:4, HDR 등)을 실제 포맷을 조회해 보존하는 대신 스케일러로 강제로 8비트 `YUV420P`까지 다운컨버트한다(ffmpeg.rs:176-202). AV1(dav1d) 경로는 `picture.bit_depth()`를 실제로 조회해(decoder.rs:377) 이 문제가 없다. 단, DEC-002에서 확인했듯 `FfmpegDecoder`(H.264/HEVC/VP9 경로)는 현재 어떤 sidecar/cli 커맨드에서도 호출되지 않는 죽은 코드라(grep 결과 0건) 지금 당장 사용자에게 도달하는 버그는 아니다 — 향후 해당 코덱들의 픽셀 디코드를 배선하는 순간 그대로 재현될 잠복 결함으로 이해해야 한다.

---

### DEC-015: decoder thread 수를 외부 scheduler와 중복 설정
**분류**: DEC · **심각도**: Medium · **탐지**: Runtime

**나쁜 예**:
```rust
fn spawn_decoders_for_batch(files: &[PathBuf]) -> Vec<JoinHandle<()>> {
    let n_cpus = std::thread::available_parallelism().unwrap().get();
    files
        .iter()
        .map(|path| {
            let path = path.clone();
            std::thread::spawn(move || unsafe {
                let ctx = create_decoder_context();
                (*ctx).thread_count = n_cpus as i32; // 디코더 내부도 전체 CPU 수만큼 스레드 사용
                decode_file(ctx, &path);
            })
        })
        .collect()
}
```

**문제**:
- 애플리케이션 레벨에서 이미 파일 개수만큼(혹은 그 이상) 스레드를 만들어 병렬 디코드를 돌리면서, 각 디코더 인스턴스에도 `thread_count = 전체 CPU 수`를 설정하면 실제 사용 스레드 수는 "외부 병렬도 × 내부 디코더 스레드 수"로 곱연산이 되어 코어 수를 몇 배 초과하는 과다구독(oversubscription)이 발생한다.
- 컨텍스트 스위칭 오버헤드가 급증하고, 캐시 지역성이 나빠지며, 전체 처리량이 오히려 스레드를 늘리기 전보다 떨어지는 역효과가 난다.
- 특히 배치 분석(여러 파일 동시 처리)이나 여러 패널이 동시에 디코드를 요청하는 UI에서 이 문제가 반복적으로 재현된다.

**발생 조건**:
- 배치 처리, 다중 패널 동시 디코드, 백그라운드 프리페치가 외부 스레드 풀과 디코더 내부 멀티스레드 옵션(FFmpeg의 frame-threading/slice-threading, dav1d의 `n_threads`)을 동시에 최대치로 설정할 때.
- CPU 코어 수가 적은 환경(저사양 노트북, CI 컨테이너의 제한된 cgroup)일수록 영향이 크게 체감된다.

**권장**:
```rust
fn spawn_decoders_for_batch(files: &[PathBuf]) -> Vec<JoinHandle<()>> {
    let total_cpus = std::thread::available_parallelism().unwrap().get();
    let n_files = files.len().max(1);
    // 외부 병렬도(파일 수)와 내부 디코더 스레드 수의 곱이 전체 CPU 수를 넘지 않도록 배분
    let per_decoder_threads = (total_cpus / n_files).max(1);

    files
        .iter()
        .map(|path| {
            let path = path.clone();
            std::thread::spawn(move || unsafe {
                let ctx = create_decoder_context();
                (*ctx).thread_count = per_decoder_threads as i32;
                decode_file(ctx, &path);
            })
        })
        .collect()
}
```
- 외부 병렬 작업 수와 디코더 내부 스레드 수를 곱한 총 스레드 수가 가용 CPU 수를 넘지 않도록 예산을 나눈다.
- 가능하다면 단일 공용 스레드 풀(예: rayon/tokio 워커 풀)에 디코드 작업을 태스크 단위로 맡기고, 디코더 내부 멀티스레딩은 끄거나 1로 고정해 스케줄링 책임을 한 곳으로 모은다.

**탐지 방법**:
- Runtime: 배치 처리 중 `nproc`보다 훨씬 많은 활성 스레드가 관찰되는지 스레드 덤프/프로파일러로 확인.
- Static: 외부 스레드 스폰 코드와 `thread_count`(또는 `n_threads`) 설정 코드가 모두 `available_parallelism()`을 각자 독립적으로 참조하는 패턴 grep.

**예외**:
- 파일을 정말 하나씩만 순차 처리하는(외부 병렬도 = 1) 도구라면 디코더 내부 스레드 수를 전체 CPU로 설정하는 것이 맞다. 문제는 "외부 병렬도가 1보다 클 수 있는데도 내부 스레드 수 계산이 그 사실을 모른다"는 점이다.

**Bitvue 판정**: N/A — bitvue-decode 어디에도 `thread_count`/`n_threads`류 디코더 스레드 수 설정이 없다(grep 결과 0건). 외부 병렬도와 내부 디코더 스레드 수를 동시에 최대치로 설정해 과다구독을 일으키는 패턴 자체가 존재하지 않는다.

---

### DEC-016: corrupted frame flag 무시
**분류**: DEC · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn compute_frame_psnr(frame: &Frame, reference: &Frame) -> f64 {
    // frame이 에러 은닉(error concealment)으로 채워진 손상 프레임인지 확인하지 않고
    // 그대로 정상 데이터인 것처럼 메트릭 계산에 사용
    psnr(&frame.planes, &reference.planes)
}
```

**문제**:
- 디코더는 비트스트림 손상, 패킷 유실, 참조 프레임 누락 등을 만나면 프레임을 완전히 실패시키는 대신 에러 은닉(이전 프레임 복사, 회색/그레이 채움, 부분 디코드 결과)으로 프레임을 "그럴듯하게" 채우고 손상 플래그(FFmpeg `AV_FRAME_FLAG_CORRUPT`, `decode_error_flags`)만 세팅하는 경우가 많다.
- 이 플래그를 확인하지 않고 손상된 프레임을 PSNR/SSIM/VMAF 같은 화질 메트릭이나 통계 집계에 그대로 넣으면, 실제로는 "디코드 실패로 인한 아티팩트"인데 "인코딩 손실로 인한 화질 저하"처럼 잘못 해석된 결과가 리포트에 섞인다.
- 사용자가 이 리포트를 근거로 인코더/비트레이트 튜닝 판단을 내리면 잘못된 결론에 도달할 수 있다.

**발생 조건**:
- 스트리밍 중 패킷 유실을 시뮬레이션한 테스트, 손상된 파일 복구 케이스, 네트워크 캡처에서 재구성한 스트림 분석.
- 정상 파일이라도 인코더 버그나 mux 오류로 드물게 손상 프레임이 섞여 있는 경우.

**권장**:
```rust
fn compute_frame_psnr(frame: &Frame, reference: &Frame) -> Option<f64> {
    if frame.is_corrupt() { // AVFrame.flags & AV_FRAME_FLAG_CORRUPT 확인을 래핑
        return None; // 명시적으로 "이 프레임은 신뢰할 수 없음"을 표현
    }
    Some(psnr(&frame.planes, &reference.planes))
}

fn aggregate_report(results: &[Option<f64>]) -> Report {
    let valid: Vec<f64> = results.iter().filter_map(|r| *r).collect();
    Report {
        mean_psnr: mean(&valid),
        corrupted_frame_count: results.iter().filter(|r| r.is_none()).count(),
        // 손상 프레임 수를 통계에서 숨기지 않고 별도로 노출
    }
}
```
- 프레임을 메트릭 계산이나 통계에 넣기 전에 손상 플래그를 확인하고, 손상된 프레임은 계산에서 제외하거나 별도 카테고리로 분리한다.
- 손상 프레임 개수/비율을 리포트에 명시적으로 노출해 사용자가 "이 구간 결과는 디코드 손상으로 신뢰도가 낮다"는 것을 알 수 있게 한다.

**탐지 방법**:
- Semantic: 프레임 데이터를 메트릭/통계 함수에 전달하는 경로에서 손상 플래그 체크가 존재하는지 데이터플로 추적.
- Manual: 의도적으로 손상시킨(패킷 일부 제거) 테스트 스트림으로 리포트에 손상 여부가 반영되는지 수동 검증.

**예외**:
- 손상 은닉 자체를 연구/시각화하려는 도구(에러 은닉 품질 비교 도구)라면 손상 프레임을 의도적으로 포함해 분석하는 것이 목적일 수 있다. 이 경우도 "이 프레임은 손상되었다"는 사실 자체는 명시적으로 드러나야 한다.

**Bitvue 판정**: Suspected — (재검증, 이전 판정은 stale) 근거였던 `calculate_frame_psnr`/`calculate_frame_ssim`/`decode_frames_up_to`(src-tauri/src/commands/quality.rs)는 `src-tauri` 삭제(`e7194cc`)로 더 이상 존재하지 않는다. 현재 PSNR/SSIM 경로(`decode_ivf_frames`, crates/bitvue-cli/src/commands/quality.rs:200-222)는 placeholder/회색 프레임 합성 코드가 없고(grep 결과 0건), `Err(NoFrame) => break`로 정상 처리한다 — DEC-001/003이 지적한 문제는 사라졌다. 다만 `DecodedFrame`(crates/bitvue-decode/src/decoder.rs:33-)에 corrupt/error-concealment 플래그 필드 자체가 없고, dav1d가 FFmpeg의 `AV_FRAME_FLAG_CORRUPT`에 해당하는 값을 노출하는지, 노출한다면 그것을 조회하는 코드가 있는지 확인하지 못했다 — 손상 스트림에 대한 실측 테스트 없이는 Confirmed/N/A를 가릴 수 없어 Suspected로 유지.

---

### DEC-017: low-delay와 reorder 동작 혼동
**분류**: DEC · **심각도**: Medium · **탐지**: Semantic

**나쁜 예**:
```rust
fn build_playback_buffer(stream_info: &StreamInfo) -> PlaybackBuffer {
    // 코덱이 B-frame을 지원한다는 이유만으로 항상 reorder 버퍼가 필요하다고 가정
    // (또는 반대로, 항상 low-delay라고 가정하고 버퍼를 아예 두지 않음)
    let depth = if stream_info.codec_supports_bframes() { DEFAULT_REORDER_DEPTH } else { 0 };
    PlaybackBuffer::with_depth(depth)
}
```

**문제**:
- "코덱이 B-frame을 지원하는가"와 "이 스트림이 실제로 low-delay로 인코딩되었는가"는 다른 질문이다. AV1/HEVC/AVC 모두 B-frame(양방향 예측)을 지원하지만, 실시간 통신용으로 인코딩된 스트림은 low-delay 프로파일(B-frame 없음 또는 reorder 없는 B-frame)을 쓸 수 있고, 반대로 "저지연"이라 불리는 설정에서도 일부 reorder가 남아있을 수 있다.
- 실제 reorder 필요 여부는 SPS/시퀀스 헤더의 `low_delay` 관련 필드나 디코더가 보고하는 `has_b_frames`/`delay` 값으로 판단해야 하는데, 이를 코덱 종류만으로 추정하면 두 방향 모두 오작동한다: (a) low-delay 스트림에 불필요한 reorder 버퍼를 둬 지연이 늘어나거나, (b) reorder가 실제로 필요한 스트림에서 버퍼를 생략해 프레임 순서가 뒤섞인 채 표시된다.

**발생 조건**:
- 실시간 통신(화상회의, 클라우드 게이밍) 캡처와 VOD 콘텐츠가 같은 코덱으로 섞여 들어오는 분석 도구.
- "이 코덱은 B-frame 있음/없음"이라는 정적 테이블을 스트림 실제 헤더 파싱 대신 사용하는 설계.

**권장**:
```rust
fn build_playback_buffer(ctx: &AVCodecContext) -> PlaybackBuffer {
    // 코덱 종류가 아니라 실제 디코더/헤더가 보고하는 delay 값을 근거로 판단
    let depth = ctx.delay.max(0) as usize;
    PlaybackBuffer::with_depth(depth)
}
```
- reorder 필요 여부·깊이는 코덱 종류가 아니라 디코더 초기화 후 실제로 보고되는 `delay`/`has_b_frames` 값(또는 시퀀스 헤더 파싱 결과)을 근거로 결정한다.
- low-delay 여부를 판단하는 로직을 스트림 헤더 파싱 단계에 한 곳으로 모아, "이 코덱은 항상 이렇다"는 가정이 여러 곳에 흩어지지 않게 한다.

**탐지 방법**:
- Semantic: reorder/버퍼링 깊이를 코덱 이름 기반 상수/테이블로 결정하는 코드 탐지.
- Runtime: low-delay 인코딩 프로파일과 일반 VOD 프로파일 양쪽의 테스트 벡터로 재생 지연과 프레임 순서를 비교 검증.

**예외**:
- 애플리케이션이 오직 하나의 알려진 인코딩 프리셋(예: 자체 캡처 파이프라인에서 항상 동일 설정으로 인코딩)만 다룬다면 정적 가정이 실용적으로 안전할 수 있다. 다만 외부에서 온 파일을 열 수 있는 기능이 조금이라도 있다면 이 가정은 깨진다.

**Bitvue 판정**: N/A — 코드명/코덱 이름 기반으로 reorder depth나 low-delay 여부를 결정하는 정적 테이블/휴리스틱이 코드베이스에 없다(관련 상수는 `MAX_FRAME_BUFFER_SIZE=16`뿐이며 이는 캐시 크기 상한이지 low-delay 판정 로직이 아니다). 이 패턴 자체가 존재하지 않는다.

---

### DEC-018: side data를 버림
**분류**: DEC · **심각도**: Medium · **탐지**: Manual

**나쁜 예**:
```rust
fn to_render_frame(frame: &AVFrame) -> RenderFrame {
    // 픽셀 plane만 복사하고 side_data(HDR 메타데이터, film grain, closed caption 등)는 버림
    RenderFrame {
        planes: copy_planes(frame),
        width: frame.width,
        height: frame.height,
    }
}
```

**문제**:
- `AVFrame`은 픽셀 데이터 외에도 `side_data`(HDR 마스터링 디스플레이 메타데이터, Content Light Level, Dolby Vision RPU, AV1 film grain 파라미터, closed caption, active format description 등)를 프레임에 실어 나른다.
- 프레임을 다른 표현으로 변환/복사하는 경로에서 이 side data를 명시적으로 옮기지 않으면 조용히 유실된다. film grain 파라미터가 사라지면 AV1 디코드 결과가 인코더 의도(그레인 합성 후 화면)와 다르게 표시되고, HDR 메타데이터가 사라지면 색 변환/톤매핑 단계가 잘못된 기본값을 쓴다.
- 분석기의 핵심 가치 중 하나가 "이 메타데이터가 실제로 스트림에 어떻게 들어있는지 보여주는 것"이라면, side data 유실은 기능 결손으로 직결된다.

**발생 조건**:
- hw→sw 프레임 전송(DEC-007), 픽셀 포맷 변환(DEC-013), 프레임을 자체 내부 구조체로 감싸는 모든 변환 지점에서 반복적으로 재발할 수 있다.
- 특히 "픽셀만 필요하다"는 좁은 요구사항으로 작성된 유틸 함수를 다른 넓은 용도(메타데이터 패널 표시)에 재사용할 때 발현.

**권장**:
```rust
fn to_render_frame(frame: &AVFrame) -> RenderFrame {
    let mut side_data = Vec::new();
    unsafe {
        for i in 0..frame.nb_side_data {
            let sd = *frame.side_data.add(i as usize);
            side_data.push(SideDataEntry::from_raw(sd)); // 타입별로 명시적으로 보존
        }
    }
    RenderFrame {
        planes: copy_planes(frame),
        width: frame.width,
        height: frame.height,
        side_data, // HDR/film grain/캡션 등을 다음 단계로 전달
    }
}
```
- 프레임을 변환/복사하는 모든 지점에서 픽셀뿐 아니라 side data도 함께 옮기는 것을 기본으로 하고, 특정 side data 타입을 의도적으로 버릴 때만 그 이유를 주석으로 남긴다.
- 분석기의 메타데이터 표시 패널(HDR 정보, film grain 패널 등)은 원본 side data를 직접 참조하도록 설계해 중간 변환 단계에서의 유실을 구조적으로 방지한다.

**탐지 방법**:
- Manual: HDR10/Dolby Vision/film grain이 포함된 테스트 파일을 열어 해당 메타데이터 패널이 실제로 값을 표시하는지 수동 확인.
- Structural: `AVFrame`을 내부 구조체로 변환하는 모든 함수를 나열하고 각각이 `side_data`를 다루는지 여부를 체크리스트로 감사.

**예외**:
- 순수 픽셀 비교(예: 두 디코더의 픽셀 정확도만 비교하는 conformance 테스트)가 목적인 좁은 유틸이라면 side data를 의도적으로 무시하는 것이 맞다. 이 경우 함수 이름/문서에 "픽셀 전용"임을 명시해 다른 용도로 오용되지 않게 한다.

**Bitvue 판정**: Suspected — 디코드 프레임 경로에는 `side_data`/film grain/HDR-SEI 추출이 없다(`DecodedFrame`에 side_data 필드 없음, `Av1Decoder`는 dav1d가 내부에서 적용하는 `apply_grain` 토글만 노출, decoder.rs:311-320). 다만 HDR/SEI류 메타데이터는 bitvue-hevc/sps.rs, bitvue-av1-codec/sequence.rs 같은 별도 비트스트림 파서 크레이트가 파싱하는 것으로 보여, 프레임 side_data 경유가 아닌 의도된 아키텍처 분리일 수도 있다 — UI까지 실제로 연결되는지는 확인하지 못했다.

---

### DEC-019: color metadata 전파 누락
**분류**: DEC · **심각도**: High · **탐지**: Semantic

**나쁜 예**:
```rust
fn frame_to_srgb(frame: &AVFrame) -> Vec<u8> {
    // color_primaries, color_trc, colorspace, color_range를 조회하지 않고
    // 항상 BT.709 + limited range라고 가정하고 변환
    yuv_to_rgb_bt709_limited(&frame.planes())
}
```

**문제**:
- 프레임의 실제 색공간 메타데이터(`color_primaries`: BT.709/BT.2020/etc, `color_trc`: 감마 커브(SDR)/PQ/HLG(HDR), `colorspace`(YUV 변환 행렬): BT.601/BT.709/BT.2020-NCL, `color_range`: limited(16-235)/full(0-255))를 조회하지 않고 고정값을 가정하면, 이 가정과 실제 스트림이 다를 때 색이 통째로 틀어진다.
- 특히 BT.2020(HDR/UHD 콘텐츠)을 BT.709로 잘못 변환하면 채도가 과도하게 빠지거나 색조가 밀리는 형태로 나타나고, limited range를 full range로(또는 그 반대로) 잘못 가정하면 검은색이 회색으로 뜨거나 하이라이트가 클리핑된다.
- 이런 오류는 "화질이 이상하다"는 모호한 증상으로만 드러나 원인 규명(디코드 문제 vs 인코딩 문제 vs 렌더링 색공간 문제)에 시간이 오래 걸린다.

**발생 조건**:
- BT.2020/HDR 콘텐츠, PC에서 캡처한 full-range 콘텐츠, 방송용 limited-range 콘텐츠가 섞여 들어오는 모든 분석 도구.
- 색공간 정보가 시퀀스 헤더/컨테이너 메타데이터 어디에도 명시되지 않은 파일(unspecified)에서 "합리적 기본값 추정" 로직 자체가 없는 경우.

**권장**:
```rust
fn frame_to_srgb(frame: &AVFrame) -> Vec<u8> {
    let primaries = resolve_primaries(frame.color_primaries); // AVCOL_PRI_UNSPECIFIED면 컨테이너/해상도 기반 추정 규칙 적용
    let trc = resolve_trc(frame.color_trc);
    let matrix = resolve_matrix(frame.colorspace);
    let range = resolve_range(frame.color_range); // AVCOL_RANGE_UNSPECIFIED 처리 포함

    yuv_to_rgb(&frame.planes(), ColorParams { primaries, trc, matrix, range })
}
```
- 색공간 변환 전에 반드시 프레임(또는 시퀀스 헤더)의 `color_primaries`/`color_trc`/`colorspace`/`color_range`를 조회하고, 이를 변환 파라미터로 명시적으로 사용한다.
- `UNSPECIFIED`인 경우를 위한 추정 규칙(예: 해상도 기반 BT.601 vs BT.709 휴리스틱, ITU-R 권고 기본값)을 한 곳에 문서화해 일관되게 적용한다.
- 원본 색공간 메타데이터는 UI에도 노출해 사용자가 "이 파일이 실제로 어떤 색공간으로 태그되어 있는지" 확인할 수 있게 한다.

**탐지 방법**:
- Semantic: YUV→RGB 변환 함수 인자에 색공간 파라미터가 상수로 고정되어 있고 프레임 메타데이터 조회가 없는 코드 탐지.
- Runtime: BT.2020/PQ, BT.709/SDR, full-range/limited-range 각각의 테스트 벡터로 변환 결과 색상값을 참조 구현(FFmpeg CLI 등)과 비교.

**예외**:
- 입력이 단일하고 검증된 파이프라인에서만 생성되는(예: 항상 BT.709 limited로만 인코딩하는 자체 캡처 도구) 경우 고정 가정이 실용적으로 안전할 수 있으나, 외부 파일을 여는 기능이 있다면 이 가정은 반드시 깨진다.

**Bitvue 판정**: Confirmed — `crates/bitvue-decode/src/yuv.rs`의 `yuv_to_rgb_pixel`(406-434)이 BT.601 계수를 무조건 하드코딩하며, `color_primaries`/`color_trc`/`colorspace`/`color_range`를 조회하는 코드가 bitvue-decode 어디에도 없다(grep 결과 0건). `DecodedFrame` 구조체 자체에도 색공간 필드가 없다. SPS/시퀀스 헤더 파서(bitvue-hevc, bitvue-av1-codec)는 이 값들을 별도로 파싱하지만 변환 함수로 전달되지 않는다.

---

### DEC-020: decoder fallback이 조용히 품질·성능을 바꿈
**분류**: DEC · **심각도**: High · **탐지**: Manual

**나쁜 예**:
```rust
fn init_decoder(codec_id: AVCodecID) -> *mut AVCodecContext {
    unsafe {
        if let Some(hw_ctx) = try_init_hwaccel(codec_id) {
            return hw_ctx;
        }
        // hwaccel 초기화 실패 시 아무 알림 없이 sw 디코더로 조용히 폴백
        init_sw_decoder(codec_id)
    }
}
```

**문제**:
- 하드웨어 가속 초기화 실패(드라이버 미지원, 세션 수 초과, 지원하지 않는 프로파일/레벨) 시 소프트웨어 디코드로 폴백하는 것 자체는 합리적이지만, 이를 사용자/로그/분석 결과 어디에도 알리지 않으면 같은 파일이 환경에 따라 다른 경로로 디코드되었다는 사실이 완전히 묻힌다.
- hw와 sw 디코더는 특히 에러 은닉, 후처리 필터(디블록킹 강도 근사), 부동소수점 vs 고정소수점 연산 등에서 픽셀 단위로 미세하게 다른 결과를 낼 수 있다. 화질 메트릭(PSNR/VMAF)을 비교하는 분석 도구에서 실행마다 결과가 미세하게 달라지면 "측정 자체가 신뢰할 수 없다"는 인상을 준다.
- 성능 측면에서도, hw→sw 폴백은 디코드 속도를 수 배 저하시킬 수 있는데 이를 사용자가 인지하지 못하면 "이 파일은 원래 느리다"는 잘못된 결론을 내리게 된다.
- 더 넓게는, 디코더 자체가 하나 이상 존재하는 상황(예: 여러 AV1 디코더 바이너리, 혹은 코덱별로 스펙 준수 수준이 다른 대체 구현)에서 어떤 디코더가 실제로 이 결과를 만들었는지 기록하지 않으면 재현성이 깨진다.

**발생 조건**:
- hwaccel을 시도하고 실패 시 sw로 폴백하는 모든 초기화 경로.
- 환경(OS, GPU 드라이버 버전, 동시 세션 수)에 따라 폴백 여부가 달라지는 경우, 동일 파일에 대해 CI와 로컬 개발 환경에서 다른 결과가 나와 "재현 안 되는 버그"로 오인되기 쉽다.

**권장**:
```rust
struct DecoderInit {
    ctx: *mut AVCodecContext,
    backend: DecoderBackend, // Hardware(VAAPI/VideoToolbox/...) 또는 Software
}

fn init_decoder(codec_id: AVCodecID) -> DecoderInit {
    unsafe {
        if let Some(hw_ctx) = try_init_hwaccel(codec_id) {
            log::info!("hwaccel 디코더로 초기화됨: {codec_id:?}");
            return DecoderInit { ctx: hw_ctx, backend: DecoderBackend::Hardware };
        }
        log::warn!("hwaccel 초기화 실패, sw 디코더로 폴백: {codec_id:?}");
        DecoderInit { ctx: init_sw_decoder(codec_id), backend: DecoderBackend::Software }
    }
}
```
- 폴백이 발생하면 로그에 명확히 남기고, 분석 결과(리포트, 메트릭 출력 메타데이터)에 실제로 사용된 디코더 백엔드를 함께 기록한다.
- 화질 비교처럼 결과의 정밀한 재현성이 중요한 기능에서는 폴백을 자동으로 허용하지 않고, "hwaccel 요청했으나 사용 불가"를 명시적 에러로 만들거나 사용자에게 확인을 구하는 옵션을 제공한다.
- 여러 디코더 구현(예: 참조 디코더 vs 최적화된 디코더)을 선택할 수 있는 도구라면 실제 사용된 구현의 이름/버전을 결과에 항상 기록한다.

**탐지 방법**:
- Manual: hwaccel을 강제로 실패시키는 환경(드라이버 없는 CI 컨테이너 등)에서 동일 파일을 분석해 결과 리포트에 백엔드 정보가 남는지, 값이 달라지는지 수동 비교.
- Structural: 폴백 분기(`try_init_hwaccel` 실패 시 경로)에 로깅/메타데이터 기록 호출이 있는지 코드 리뷰 체크.

**예외**:
- 화질 수치의 절대적 재현성이 중요하지 않은 용도(단순 미리보기 재생)라면 조용한 폴백이 사용자 경험상 오히려 바람직할 수 있다. 다만 이 경우도 최소한 디버그 로그에는 남겨 문제 발생 시 추적 가능하게 하는 것이 안전하다.

**Bitvue 판정**: N/A — DEC-007과 동일한 이유로 hwaccel 경로 자체가 코드베이스에 없으므로 hw→sw 조용한 폴백이라는 시나리오가 성립하지 않는다.
