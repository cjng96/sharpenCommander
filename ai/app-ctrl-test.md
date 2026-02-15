# Controller Unit Test Document

이 문서는 `rust/src/ui/` 디렉토리에 정의된 컨트롤러들의 유닛 테스트 구현 상태를 관리합니다.

## MainCtrl (`rust/src/ui/main_ctrl.rs`)

### 탐색 및 기본 동작 (`test_main_ctrl_navigation`)
- 설명: 파일 목록 갱신, 항목 포커스, 상/하 선택 이동, 디렉토리 진입 및 부모 디렉토리 이동 기능 확인
- 상태: 구현됨
- 위치: `rust/src/ui/main_ctrl.rs`

### 선택 범위 제한 (`test_main_ctrl_selection_bounds`)
- 설명: 목록의 경계에서 선택 이동 시 인덱스 범위 초과 방지 확인
- 상태: 구현됨
- 위치: `rust/src/ui/main_ctrl.rs`

### 목록 필터링 (`test_main_ctrl_refresh_filtering`)
- 설명: 특수 폴더(`.dcdata`) 필터링 확인
- 상태: 구현됨
- 위치: `rust/src/ui/main_ctrl.rs`

---

## FindCtrl (`rust/src/ui/find_ctrl.rs`)

### 기본 동작 (`test_find_ctrl_basic`)
- 설명: 파일 목록 관리, 선택 이동, 파일 내용 로드 기능 확인
- 상태: 구현됨

---

## GrepCtrl (`rust/src/ui/grep_ctrl.rs`)

### 기본 동작 (`test_grep_ctrl_basic`)
- 설명: 검색 결과 목록 관리, 선택 이동 기능 확인
- 상태: 구현됨

---

## GitStatusCtrl (`rust/src/ui/git_status_ctrl.rs`)

### 탐색 및 내용 로드 (`test_git_status_ctrl_navigation`)
- 설명: Git 상태 항목 목록 탐색 및 항목 선택 시 diff 내용 로드 확인 (Git repo 환경)
- 상태: 구현됨
- 위치: `rust/src/ui/git_status_ctrl.rs`

---

## GitCommitCtrl (`rust/src/ui/git_commit_ctrl.rs`)

### 기본 동작 (`test_git_commit_ctrl_basic`)
- 설명: 스테이징된 파일 목록 및 커밋 로그 관리 확인
- 상태: 구현됨
- 위치: `rust/src/ui/git_commit_ctrl.rs`

---

## RegListCtrl (`rust/src/ui/reg_list_ctrl.rs`)

### 필터링 및 선택 (`test_reg_list_ctrl_filtering`)
- 설명: 입력 필터에 따른 목록 필터링 및 항목 선택 확인
- 상태: 구현됨
- 위치: `rust/src/ui/reg_list_ctrl.rs`

### 상태 정보 데이터 포맷팅 (`test_repo_status_info_formatting`)
- 설명: 저장소의 dirty 여부, ahead/behind 카운트가 올바르게 포맷팅되는지 확인
- 상태: 구현됨
- 위치: `rust/src/git.rs`

### Pull 완료 후 상태 자동 갱신 (`test_reg_list_ctrl_refresh_after_pull`)
- 설명: 개별 또는 일괄 Pull 작업이 완료되었을 때, 해당 저장소의 Git 상태(ahead/behind 등)를 다시 확인하는 작업이 시작되는지 확인
- 상태: 구현됨
- 위치: `rust/src/ui/reg_list_ctrl.rs`

### 항목 표시 정보 관리 (`test_reg_list_ctrl_status_update`)
- 설명: 비동기적으로 수신된 StatusEvent가 status_infos에 올바르게 저장되고 목록 정렬에 반영되는지 확인 (dirty 여부 또는 ahead 카운트에 따른 표시 색상 및 '*' 마커 로직 포함)
- 상태: 구현됨
- 위치: `rust/src/ui/reg_list_ctrl.rs`

### 초기 상태값 로드 (`test_reg_list_ctrl_initial_status_check`)
- 설명: 화면 진입 시 모든 저장소에 대해 비동기 Git 상태 확인 작업이 시작되는지 확인
- 상태: 구현됨
- 위치: `rust/src/ui/reg_list_ctrl.rs`

### 표시 명칭 생성 로직 (`test_reg_item_display_name`)
- 설명: 저장소의 이름, 상태값, 경로를 조합하여 '이름 상태값 (상위경로)' 형태 또는 '경로' 형태의 표시 명칭이 올바르게 생성되는지 확인 (이름이 경로의 노드값과 일치하면 경로에서 해당 값 제외)
- 상태: 구현됨
- 위치: `rust/src/config.rs`

### 항목 변경 시 상세 정보 초기화 (`test_reg_list_ctrl_clear_detail`)
- 설명: 목록에서 선택된 항목(경로)이 변경될 때 오른쪽 상세 정보 패널(status_lines 등)이 즉시 비워지는지 확인
- 상태: 구현됨
- 위치: `rust/src/ui/reg_list_ctrl.rs`

### 상태 업데이트 및 정렬 (`test_reg_list_ctrl_status_update`)
- 설명: 비동기 Git 상태 확인 및 결과에 따른 자동 정렬 로직 검증 (변경된 저장소가 상단에 위치하는지 확인)
- 상태: 구현됨
- 위치: `rust/src/ui/reg_list_ctrl.rs`

---

## GotoCtrl (`rust/src/ui/goto_ctrl.rs`)

### 필터링 기능 (`test_goto_ctrl_filtering`)
- 설명: 입력 필터에 따른 저장소/파일/디렉토리 필터링 확인
- 상태: 구현됨
- 위치: `rust/src/ui/goto_ctrl.rs`

### 가중치 기반 정렬
- 설명: 검색 일치도 및 타입 우선순위에 따른 정렬 확인
- 상태: (계획) 미구현
- 위치: -

### 선택 항목 경로 획득 (`test_goto_ctrl_focus_item_path`)
- 설명: 현재 선택된 아이템(Repo, LocalDir, LocalFile)의 절대 경로를 올바르게 반환하는지 확인
- 상태: 구현됨
- 위치: `rust/src/ui/goto_ctrl.rs`