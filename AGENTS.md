# Repository Guidelines

## Project Structure & Module Organization
- Current implementation lives in `rust/`.
- Core Rust entry points are `rust/src/main.rs` and `rust/src/lib.rs`.
- Feature modules are split under `rust/src/` (for example `app.rs`, `git.rs`, `config.rs`, `system.rs`, `util.rs`).
- UI-specific code is organized in `rust/src/ui/` with paired `*_ctrl.rs` and `*_ui.rs` files.
- Integration and behavior tests are in `rust/tests/integration_test.rs`.
- Legacy Python code remains under `sharpenCommander/` and root Python packaging files (`setup.py`, `requirements.txt`) are kept for backward compatibility only; prefer Rust for new work.

## Build, Test, and Development Commands
Run from `rust/` unless noted.
- `cargo run -- <args>`: run the app locally.
- `cargo test`: run unit and integration tests.
- `cargo build`: compile debug binaries.
- `cargo build --release`: produce optimized binary.
- `./r <args>`: watch `src/` and rerun via `watchexec` for iterative development.

## Coding Style & Naming Conventions
- Use Rust 2021 idioms and keep code `cargo fmt` clean before opening a PR.
- Indentation: 4 spaces; keep lines readable and avoid large monolithic functions.
- Naming:
  - files/modules/functions: `snake_case`
  - types/traits: `PascalCase`
  - constants: `SCREAMING_SNAKE_CASE`
- Follow existing pairing patterns like `goto_ctrl.rs` + `goto_ui.rs` when adding UI flows.

## Testing Guidelines
- Add tests for new behavior in `rust/tests/integration_test.rs` or local module tests where appropriate.
- Test names should describe behavior (`test_add_to_gitignore`, `match_disorder_basic`).
- Ensure `cargo test` passes before committing.

## Commit & Pull Request Guidelines
- Follow the repository’s conventional style seen in history: `feat: ...`, `fix: ...`, `refactor: ...`, `test: ...` (optional scope like `rust:` is acceptable).
- Keep commits focused and atomic.
- PRs should include:
  - brief problem/solution summary
  - linked issue (if any)
  - testing evidence (commands run, e.g. `cargo test`)
  - screenshots or terminal captures for UI-visible changes.

# 프로그램의 목적
이 프로그램(`sc`)은 CLI 환경에서 효율적인 파일 시스템 탐색과 Git 관리를 지원하는 도구입니다.
너는 스펙문서 작성과 테스트 코드작성을 꼭 챙기는 프로 개발자야!

# 지침 및 개요
- 코드 작성: 읽기 쉽고 유지보수하기 쉽게 간결하게 작성. 로직 코드는 최대한 유닛 테스트를 만들 수 있게 작성
- 코드 수정: 꼭 필요한 부분만 수정. 기존 프로젝트 컨벤션과 스타일을 엄격히 준수
- 함수화 및 재활용: 코드가 함수화에 적합하면 함수로 감싸고, 중복된 코드가 없게 기존 함수를 최대한 재활용
- 커밋: 자동으로 커밋하지 말고, 요청 시에만 이전 작업 내역 바탕으로 커밋 메시지 작성 후 수행
- 문서 수정: 문서도 꼭 필요한 부분만 최소한으로 수정하고, 수정후 수정된 내역을 보여줘.
- 스펙문서는 특별히 요청하지 않으면 지침 및 개요 항목은 수정하지 않는다.
- ui관련된 코드: ui/폴더 아래에 위치시키고, ui/아래에 있는 코드는 최소화한다.(로직 코드는 최대한 다른 파일로 보낸다)

## 작업 진행 절차
- 버그 수정, 기능 추가 혹은 개선 요청 시에는 스펙문서(*-spec.md)를 먼저 갱신
- 스펙문서에 수정한 내용을 바탕으로 테스트 문서(*-test.md)에 테스트 방법 작성
- 이후 unit test를 구현 후 기능 구현 후 unit test통과 할때까지 노력
  
## spec문서(*-spec.md)
- *-ui-spec.md파일은 화면별로 프론트엔드는 화면 중심으로 서술
- 기능 구현은 언제나 스펙문서의 내용을 기반으로 한다.
- 스펙의 기능이 구현완료되면, 체크박스를 체크상태로 변경

## test문서(*-test.md)
- 모든 구현 기능은 테스트 케이스가 등록되어 있어야한다
- 테스트케이스는 모두 구현하고, 구현 위치를 같이 기재한다.

## 유닛 테스트
- 코드를 수정하기전에 테스트 항목 및 전략을 *-test.md파일에 작성 후 테스트 작성 해서 수행
- `*-test.md` 문서에 소스 파일/클래스 중심으로 유닛 테스트 케이스를 나열하고, 유닛 테스트 구현 상태를 추가로 명시
  - 유닛 테스트가 구현되었다면 해당 유닛 테스트 위치 명시
  - 유닛 테스트가 아직 구현되지 않았다면 구현
  - 유닛 테스트를 구현할 수 없다면 이유 서술

