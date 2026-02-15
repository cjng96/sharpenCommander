## GitHistory Screen 테스트

| 테스트 항목 | 목적 | 구현 상태 | 구현/검증 위치 |
|---|---|---|---|
| GitHistory 데이터 소스(gitoxide) | commit list/detail 조회가 외부 `git` 프로세스 대신 `gix` 경로를 타는지 확인 | 구현됨(유닛 테스트) | `rust/src/git.rs` (`test_commit_history_and_detail_with_gix`) |
| 기본 레이아웃 배치 | 상단 filter, 그 아래 commit 목록(약 10줄), 하단 detail 패널 배치 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/git_history_ui.rs` |
| Filter 입력 높이 | Filter 입력 영역이 1줄 높이로 유지되는지 확인 | 구현됨(유닛 테스트+수동 확인) | `rust/src/ui/git_history_ui.rs` (`test_filter_input_height_is_one_line`) |
| 진입 조건 (Git 저장소 여부) | Git 저장소가 아닌 경우 진입 차단 및 경고 동작 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/main_ui.rs`, `rust/src/ui/git_history_ui.rs` |
| 필터 입력 UI | 상단 filter 입력란에서 문자열 입력/삭제가 가능한지 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/git_history_ui.rs` |
| 필터링 대상 | filter 문자열로 `author`, `subject`가 함께 필터링되는지 확인 | 구현됨(유닛 테스트) | `rust/src/ui/git_history_ctrl.rs` (`test_git_history_ctrl_filter_by_author_and_subject`) |
| 섹션 제목 라인 스타일 | GitHistory의 `Filter`, `Commits`, `Detail` 제목 라인이 전체 폭(full width) 배경색/글자색으로 렌더링되는지 확인 | 구현됨(유닛 테스트+수동 확인) | `rust/src/ui/git_history_ui.rs` (`test_section_title_line_uses_colors`) |
| Diff 색상 규칙(git diff 동일) | 메타/파일헤더/hunk/추가/삭제 라인이 `git diff` 기본 색상 규칙으로 렌더링되는지 확인 | 구현됨(유닛 테스트) | `rust/src/ui/common.rs` (`test_format_diff_lines_git_diff_headers`, `test_format_diff_lines_git_diff_hunk_and_changes`) |
| Diff 컨텍스트 3줄 | 커밋 상세 diff hunk가 변경 라인 전후 3줄 컨텍스트를 포함하는지 확인 | 구현됨(유닛 테스트) | `rust/src/git.rs` (`test_commit_detail_contains_three_context_lines`) |
| Diff 파일 단위 표시 | 커밋 상세 diff에 디렉토리(tree) 항목이 표시되지 않고 파일 단위만 표시되는지 확인 | 구현됨(유닛 테스트) | `rust/src/git.rs` (`test_commit_detail_does_not_emit_tree_entries`) |
| 커밋 선택 이동 | `j/k`, `Up/Down`으로 커밋 선택 이동 및 상세 패널 갱신 확인 | 구현됨(유닛 테스트+수동 확인) | `rust/src/ui/git_history_ctrl.rs` (`test_git_history_ctrl_navigation_bounds`), `rust/src/ui/git_history_ui.rs` |
| 상세 스크롤 | 상세 diff 패널 스크롤(`Ctrl+J/K`, wheel) 동작 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/git_history_ui.rs` |
| 화면 이동 | `Q/Esc/Left` 메인 복귀, `C` GitStage 이동 확인 | 구현됨(수동 확인 필요) | `rust/src/ui/git_history_ui.rs` |
