# Controller Unit Test Document

이 문서는 `rust/src/ui/` 디렉토리에 정의된 컨트롤러들의 유닛 테스트 구현 상태를 관리합니다.

## MainCtrl (`rust/src/ui/main_ctrl.rs`)

`MainCtrl`은 프로그램의 메인 화면 로직을 담당하며, 파일 시스템 탐색 및 선택 기능을 관리합니다.

| 테스트 케이스 | 설명 | 상태 | 위치 |
| :--- | :--- | :--- | :--- |
| 탐색 및 기본 동작 (`test_main_ctrl_navigation`) | 파일 목록 갱신, 항목 포커스, 상/하 선택 이동, 디렉토리 진입 및 부모 디렉토리 이동 기능 확인 | 구현됨 | `rust/src/ui/main_ctrl.rs` (mod tests) |
| 선택 범위 제한 (`test_main_ctrl_selection_bounds`) | 목록의 첫 번째 항목에서 위로 이동하거나 마지막 항목에서 아래로 이동 시 인덱스가 범위를 벗어나지 않는지 확인 | 구현됨 | `rust/src/ui/main_ctrl.rs` (mod tests) |
| 목록 필터링 (`test_main_ctrl_refresh_filtering`) | `.dcdata`와 같은 내부 데이터 폴더가 목록에서 정상적으로 제외되는지 확인 | 구현됨 | `rust/src/ui/main_ctrl.rs` (mod tests) |

## Main UI (`rust/src/ui/main_ui.rs`)

- [x] `E` 키: 선택된 파일을 `edit_app`으로 연다 (`rust/src/ui/main_ui.rs` - `test_main_e_opens_selected_file`)
- [x] `E` 키: 선택된 폴더를 `edit_app`으로 연다 (`rust/src/ui/main_ui.rs` - `test_main_e_opens_selected_directory`)
- [x] `E` 키: 최상단 `..` 선택 시 현재 작업 폴더(`cwd`)를 `edit_app`으로 연다 (`rust/src/ui/main_ui.rs` - `test_main_e_on_parent_entry_opens_current_directory`)

## GotoCtrl (`rust/src/ui/goto_ctrl.rs`)

`GotoCtrl`은 등록된 저장소 및 로컬 파일/디렉토리로의 빠른 이동 기능을 담당합니다.

| 테스트 케이스 | 설명 | 상태 | 위치 |
| :--- | :--- | :--- | :--- |
| 필터링 기능 (`test_goto_ctrl_filtering`) | 입력된 필터 문자열에 따라 저장소 목록이 정확하게 필터링되는지 확인 | 구현됨 | `rust/src/ui/goto_ctrl.rs` (mod tests) |
| 가중치 기반 정렬 | (계획) 필터 일치도 및 타입 우선순위에 따른 정렬 로직 검증 | 미구현 | - |
