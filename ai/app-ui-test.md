## GitHistory Screen 테스트

| 테스트 항목 | 목적 | 구현 상태 | 구현/검증 위치 |
|---|---|---|---|
| 기본 레이아웃 배치 | 상단 filter, 그 아래 commit 목록(약 10줄), 하단 detail 패널 배치 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/git_history_ui.rs` |
| 진입 조건 (Git 저장소 여부) | Git 저장소가 아닌 경우 진입 차단 및 경고 동작 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/main_ui.rs`, `rust/src/ui/git_history_ui.rs` |
| 필터 입력 UI | 상단 filter 입력란에서 문자열 입력/삭제가 가능한지 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/git_history_ui.rs` |
| 필터링 대상 | filter 문자열로 `author`, `subject`가 함께 필터링되는지 확인 | 구현됨(유닛 테스트) | `rust/src/ui/git_history_ctrl.rs` (`test_git_history_ctrl_filter_by_author_and_subject`) |
| 커밋 선택 이동 | `j/k`, `Up/Down`으로 커밋 선택 이동 및 상세 패널 갱신 확인 | 구현됨(유닛 테스트+수동 확인) | `rust/src/ui/git_history_ctrl.rs` (`test_git_history_ctrl_navigation_bounds`), `rust/src/ui/git_history_ui.rs` |
| 상세 스크롤 | 상세 diff 패널 스크롤(`Ctrl+J/K`, wheel) 동작 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/git_history_ui.rs` |
| 화면 이동 | `Q/Esc/Left` 메인 복귀, `C` GitStage 이동 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/git_history_ui.rs` |
