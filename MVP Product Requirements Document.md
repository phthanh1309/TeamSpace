# TeamSpace — MVP Product Requirements Document

**Version:** 1.0  
**Status:** Ready for implementation planning  
**Product:** TeamSpace  
**Positioning:** Project-first Collaboration for Student Teams

---

# PART I — PRODUCT REQUIREMENTS DOCUMENT

## 1. Product Vision

TeamSpace là nền tảng cộng tác giúp các nhóm học tập quản lý toàn bộ quy trình thực hiện dự án trong một không gian thống nhất.

Thay vì phân tán công việc giữa Messenger, Google Drive, Canva và các ứng dụng quản lý task, TeamSpace kết nối:

- Dự án.
- Thành viên.
- Nhiệm vụ.
- Thảo luận.
- Tài liệu.
- Sản phẩm bàn giao.
- Đánh giá và phê duyệt.

### Core value proposition

**Stop chasing messages. Start completing projects.**

TeamSpace không đơn thuần giúp nhóm giao tiếp. TeamSpace giúp nhóm hoàn thành công việc.

## 2. Target Users

### Primary audience

Student teams có 3–10 thành viên:

- Nhóm làm bài tập.
- Nhóm thuyết trình.
- Nhóm nghiên cứu.
- Nhóm tổ chức sự kiện.
- Nhóm thiết kế và sáng tạo.

### User roles

**Project Owner**
- Tạo dự án.
- Mời thành viên.
- Quản lý thông tin dự án.
- Có quyền quản lý thành viên và cài đặt dự án.

**Project Member**
- Xem dự án.
- Nhận nhiệm vụ.
- Cập nhật tiến độ.
- Bình luận.
- Gửi sản phẩm bàn giao.

**Reviewer**
- Xem sản phẩm được gửi.
- Đưa ra feedback.
- Yêu cầu chỉnh sửa.
- Phê duyệt sản phẩm.

Một người có thể giữ nhiều vai trò.

## 3. Core User Journey

1. Người dùng đăng nhập.
2. Tạo dự án mới.
3. Mời thành viên.
4. Tạo nhiệm vụ.
5. Phân công người phụ trách.
6. Thành viên thực hiện công việc.
7. Trao đổi trong nhiệm vụ.
8. Gửi sản phẩm bàn giao.
9. Reviewer phản hồi hoặc phê duyệt.
10. Hoàn thành dự án.

Đây là user journey quan trọng nhất của MVP.

## 4. MVP Modules

### 4.1 Authentication

Chức năng:
- Đăng ký và đăng nhập.
- Đăng nhập Google.
- Đăng xuất.
- Quản lý session.
- Xem thông tin tài khoản.

MVP có thể triển khai email/password trước, sau đó bổ sung OAuth Google. Facebook OAuth không phải điều kiện để kiểm chứng workflow.

Acceptance criteria:
- Người dùng có thể tạo tài khoản.
- Người dùng có thể đăng nhập và đăng xuất.
- API được bảo vệ bằng authentication.
- Người dùng không thể truy cập dữ liệu của tài khoản khác.
- Session hết hạn phải được xử lý an toàn.

### 4.2 Projects

Chức năng:
- Tạo dự án.
- Chỉnh sửa dự án.
- Lưu mô tả.
- Mời thành viên.
- Xem danh sách dự án.
- Xem Project Overview.
- Lưu trạng thái dự án.

Project fields:
- id
- name
- description
- owner_id
- status
- created_at
- updated_at

Acceptance criteria:
- Người dùng có thể tạo dự án.
- Thành viên được mời có thể truy cập dự án.
- Người không thuộc dự án không thể xem dữ liệu.
- Project Overview hiển thị task và tiến độ tổng quan.
- Có thể lưu trữ hoặc đóng dự án.

### 4.3 Tasks

Chức năng:
- Tạo task.
- Chỉnh sửa task.
- Xóa task.
- Giao task cho thành viên.
- Đặt deadline.
- Checklist.
- Cập nhật trạng thái.
- Lọc task theo người phụ trách và trạng thái.

Task statuses:
- Todo
- In Progress
- Submitted
- In Review
- Approved
- Revision Requested

Task fields:
- id
- project_id
- title
- description
- assignee_id
- creator_id
- status
- priority
- due_at
- created_at
- updated_at

Acceptance criteria:
- Thành viên có quyền phù hợp có thể tạo task.
- Task thuộc đúng một project.
- Có thể giao task cho thành viên của project.
- Deadline hiển thị đúng theo múi giờ người dùng.
- Có thể cập nhật trạng thái.
- Không thể truy cập task thuộc project không có quyền tham gia.

### 4.4 Task Discussion

Chức năng:
- Bình luận trong task.
- Hiển thị người gửi và thời gian.
- Chỉnh sửa hoặc xóa bình luận của chính mình theo chính sách.
- Hiển thị các bình luận theo thứ tự thời gian.

Acceptance criteria:
- Chỉ thành viên dự án được xem và bình luận.
- Bình luận được liên kết với task.
- Hiển thị chính xác tác giả.
- Có thể phân trang khi số lượng bình luận lớn.

MVP chưa cần xây dựng chat realtime phức tạp.

### 4.5 Deliverables

Chức năng:
- Gửi link sản phẩm.
- Upload file.
- Ghi chú khi gửi.
- Liệt kê các sản phẩm đã nộp.
- Hiển thị trạng thái review.

Supported initial formats:
- PDF
- PPTX
- DOCX
- PNG
- JPG
- JPEG

Deliverable fields:
- id
- task_id
- submitted_by
- title
- description
- external_url
- storage_key
- created_at

Acceptance criteria:
- Người dùng có thể gửi URL hoặc file.
- Deliverable liên kết với đúng task.
- File được lưu ở object storage, không lưu trực tiếp trong PostgreSQL.
- Có giới hạn kích thước và kiểm tra loại file.
- Chỉ thành viên có quyền mới xem được deliverable.

### 4.6 Review Workflow

Workflow:

Assigned → In Progress → Submitted → In Review → Approved

Nếu cần chỉnh sửa:

In Review → Revision Requested → In Progress → Submitted

Chức năng:
- Gửi sản phẩm để review.
- Reviewer nhận xét.
- Yêu cầu chỉnh sửa.
- Phê duyệt sản phẩm.

Acceptance criteria:
- Deliverable có trạng thái review.
- Reviewer có thể gửi feedback.
- Thành viên nhận được thông tin phản hồi.
- Lưu lại lịch sử các lần review.
- Chỉ người có quyền mới có thể approve.

### 4.7 Home Dashboard

Hiển thị:
- Dự án gần đây.
- Task được giao cho tôi.
- Deadline sắp tới.
- Task quá hạn.
- Hoạt động gần đây.

Acceptance criteria:
- Chỉ hiển thị dữ liệu người dùng được phép truy cập.
- Deadline được sắp xếp hợp lý.
- Có đường dẫn đến project và task tương ứng.
- Dashboard có trạng thái loading, empty và error.

### 4.8 Notifications

MVP notification events:
- Được thêm vào dự án.
- Được giao task.
- Có bình luận mới trong task.
- Deliverable được gửi.
- Có feedback review.
- Deliverable được phê duyệt.

Acceptance criteria:
- Thông báo liên kết với đúng đối tượng.
- Có trạng thái đã đọc/chưa đọc.
- Không gửi thông báo trùng lặp ngoài ý muốn.

### 4.9 Settings

- Ngôn ngữ: Vietnamese / English.
- Theme: Light / Dark / System.
- Notification preferences.
- Account information.
- Session management.
- About.
- Privacy Policy.
- Terms.
- Logout.

## 5. Non-functional Requirements

### Security
- Authentication và authorization ở backend.
- Kiểm tra quyền truy cập theo project membership.
- Không tin tưởng ID hoặc role gửi từ client.
- Secrets lưu trong environment variables.
- Không lưu mật khẩu dạng plaintext.
- Giới hạn request và upload phù hợp.

### Performance
- Phân trang danh sách task và discussion.
- Database indexes cho các truy vấn thường dùng.
- Không tải toàn bộ project data cùng lúc.
- Tối ưu truy vấn dashboard.

### Reliability
- Database migrations.
- Transaction cho các thao tác nhiều bước.
- Error handling thống nhất.
- Logging và tracing.
- Backup database theo khả năng của hạ tầng.

### Accessibility
- Có keyboard navigation trên desktop.
- Trạng thái focus rõ ràng.
- Độ tương phản phù hợp.
- Kích thước vùng chạm phù hợp trên mobile.

### Localization
- English và Vietnamese.
- Không hardcode text trong UI.
- Fallback về English.
- Format ngày giờ theo locale.

## 6. Out of Scope

Không triển khai trong MVP:

- Video calls.
- Voice calls.
- Full Slack-like messaging.
- Full document editor.
- Full calendar.
- AI project manager.
- Advanced analytics.
- Complex automation.
- Deep Canva API integration.
- Enterprise billing.
- Microservices.

## 7. MVP Definition of Done

MVP được xem là hoàn thành khi:

1. Người dùng có thể đăng nhập.
2. Tạo project và mời thành viên.
3. Tạo và phân công task.
4. Thành viên cập nhật trạng thái task.
5. Bình luận trong task.
6. Gửi file hoặc link sản phẩm.
7. Reviewer feedback và approve.
8. Dashboard hiển thị đúng tiến độ.
9. Quyền truy cập được kiểm tra ở backend.
10. Có thể chạy và kiểm thử workflow trên các nền tảng được chọn.

---

# PART II — PRODUCT BACKLOG

## 8. Epic Overview

| Epic | Tên | Priority |
|---|---|---|
| EPIC-01 | Foundation & Architecture | P0 |
| EPIC-02 | Authentication | P0 |
| EPIC-03 | Project Management | P0 |
| EPIC-04 | Task Management | P0 |
| EPIC-05 | Task Discussion | P0 |
| EPIC-06 | Deliverable Management | P0 |
| EPIC-07 | Review Workflow | P0 |
| EPIC-08 | Home Dashboard | P0 |
| EPIC-09 | Notifications | P1 |
| EPIC-10 | Settings & Localization | P1 |
| EPIC-11 | Kanban & Activity | P1 |
| EPIC-12 | Messaging & Advanced Features | P2 |

## 9. Detailed Backlog

### EPIC-01 — Foundation & Architecture

**US-001: Initialize Rust workspace**
- Tạo Cargo workspace.
- Thiết lập Rust toolchain.
- Cấu hình dependencies.
- Thiết lập formatting và linting.

Acceptance:
- Workspace build thành công.
- Các crate được tổ chức rõ ràng.
- CI chạy được cargo check và cargo test.

**US-002: Create shared domain crate**
- Định nghĩa entity.
- Định nghĩa domain errors.
- Định nghĩa business rules.
- Không phụ thuộc Dioxus hoặc Axum.

Acceptance:
- Domain crate có thể được test độc lập.

**US-003: Create backend application**
- Axum server.
- Tokio runtime.
- API routing.
- Configuration.
- Error handling.

Acceptance:
- Server khởi chạy.
- Health endpoint hoạt động.
- Cấu hình không chứa secrets hardcode.

**US-004: Database setup**
- PostgreSQL.
- SQLx.
- Migration system.
- Connection pool.

Acceptance:
- Có thể chạy migrations.
- Có thể kết nối database.
- Integration test sử dụng database test.

**US-005: Create Dioxus client**
- App entrypoint.
- Routing.
- Theme provider.
- UI foundation.

Acceptance:
- App chạy trên target desktop đầu tiên.
- Có thể điều hướng giữa các màn hình cơ bản.

### EPIC-02 — Authentication

**US-006: Register account**
- Form đăng ký.
- Validation.
- Password hashing.

**US-007: Login**
- Login form.
- Authentication API.
- Session handling.

**US-008: Logout**
- Revoke session.
- Clear client authentication state.

**US-009: Google OAuth**
- Provider configuration.
- OAuth callback.
- Link provider identity với user nội bộ.

Acceptance chung:
- Có thể đăng ký, đăng nhập và đăng xuất.
- Không dùng email làm định danh duy nhất cho OAuth identity.
- Kiểm thử các trường hợp session không hợp lệ.

### EPIC-03 — Project Management

**US-010: Create project**
- Project creation form.
- API.
- Database persistence.

**US-011: Edit project**
- Update name.
- Update description.
- Update status.

**US-012: Invite members**
- Add member.
- Member roles.
- Invitation acceptance.

**US-013: Project list**
- List projects.
- Filter và pagination.

**US-014: Project overview**
- Project information.
- Member list.
- Task summary.
- Progress summary.

Acceptance:
- Có thể tạo và quản lý một project với nhiều thành viên.
- Kiểm tra quyền truy cập ở mọi API liên quan.

### EPIC-04 — Task Management

**US-015: Create task**
- Title.
- Description.
- Assignee.
- Due date.
- Priority.

**US-016: Edit task**
- Update task information.

**US-017: Assign task**
- Chọn project member.
- Thay đổi assignee.

**US-018: Update task status**
- Todo.
- In Progress.
- Submitted.
- In Review.
- Approved.
- Revision Requested.

**US-019: Checklist**
- Add checklist item.
- Toggle completion.
- Reorder items.

**US-020: Task filtering**
- Filter by status.
- Filter by assignee.
- Filter by due date.

Acceptance:
- Thành viên có thể quản lý task theo quyền.
- Task không thể được gán cho người không thuộc project.
- Các thay đổi được lưu chính xác.

### EPIC-05 — Task Discussion

**US-021: Create comment**
- Comment editor.
- Comment API.

**US-022: Display comments**
- Author.
- Timestamp.
- Pagination.

**US-023: Edit/delete own comment**
- Ownership verification.
- Update/delete API.

Acceptance:
- Discussion chỉ được truy cập bởi project members.
- Người dùng chỉ chỉnh sửa/xóa bình luận theo quyền được cấp.

### EPIC-06 — Deliverable Management

**US-024: Submit external link**
- URL input.
- Title.
- Description.

**US-025: Upload file**
- Upload endpoint.
- Object storage.
- File metadata.

**US-026: Deliverable list**
- List by task.
- Show submitter.
- Show submitted time.
- Show review status.

**US-027: Deliverable detail**
- File/link information.
- Description.
- Review history.

Acceptance:
- File được lưu ngoài database.
- Có kiểm tra quyền, dung lượng và loại file.
- Không công khai file riêng tư bằng URL không kiểm soát.

### EPIC-07 — Review Workflow

**US-028: Submit for review**
- Change task/deliverable state.
- Record submission.

**US-029: Review feedback**
- Feedback editor.
- Reviewer identity.
- Timestamp.

**US-030: Request revision**
- Revision Requested status.
- Notify submitter.

**US-031: Approve deliverable**
- Approve action.
- Record reviewer.
- Record approval timestamp.

Acceptance:
- Trạng thái chỉ chuyển theo workflow hợp lệ.
- Mọi thao tác review được lưu lịch sử.
- Người không có quyền không thể approve.

### EPIC-08 — Home Dashboard

**US-032: Project overview cards**
- Recent projects.
- Project progress.

**US-033: My tasks**
- Assigned tasks.
- Task status.

**US-034: Upcoming deadlines**
- Today.
- Overdue.
- Next 7 days.

**US-035: Recent activity**
- Project updates.
- Task updates.
- Review activity.

Acceptance:
- Dashboard phản ánh dữ liệu backend.
- Có loading, empty và error states.
- Không hiển thị dữ liệu ngoài quyền truy cập.

### EPIC-09 — Notifications

**US-036: Notification creation**
- Event-based notifications.

**US-037: Notification inbox**
- List notifications.
- Read/unread.

**US-038: Notification preferences**
- Configure event types.

### EPIC-10 — Settings & Localization

**US-039: Theme settings**
- Light.
- Dark.
- System.

**US-040: Language settings**
- Vietnamese.
- English.

**US-041: Account settings**
- Account details.
- Session information.
- Logout.

### EPIC-11 — Kanban & Activity

**US-042: Kanban board**
- Columns by status.
- Move task.
- Persist status.

**US-043: Project activity timeline**
- Task created.
- Task updated.
- Deliverable submitted.
- Review completed.

### EPIC-12 — Messaging & Advanced Features

**US-044: Direct messaging**

**US-045: Group messaging**

**US-046: Search**

**US-047: Project templates**

**US-048: Document management**

**US-049: Advanced integrations**

---

# PART III — IMPLEMENTATION ROADMAP

## 10. Development Phases

### Phase 0 — Validation
- Interview 5–10 student teams.
- Prototype key screens.
- Validate project workflow.
- Finalize MVP requirements.

Deliverable: validated requirements and prototype.

### Phase 1 — Technical Foundation
- US-001 through US-005.
- CI.
- Database.
- API foundation.
- Client foundation.

Deliverable: application skeleton running.

### Phase 2 — Project & Task Workflow
- US-006 through US-020.
- Authentication.
- Projects.
- Membership.
- Tasks.
- Checklist.

Deliverable: teams can create projects and manage work.

### Phase 3 — Collaboration & Review
- US-021 through US-031.
- Discussion.
- Deliverables.
- Review.

Deliverable: core USP works end-to-end.

### Phase 4 — Dashboard & Beta
- US-032 through US-041.
- Dashboard.
- Notifications.
- Settings.
- Localization.
- Beta testing.

Deliverable: MVP usable by real student teams.

### Phase 5 — Expansion
- US-042 onward.
- Kanban.
- Activity.
- Messaging.
- Templates.
- Search.

Deliverable: broader collaboration experience.

## 11. Suggested First Sprint

Sprint objective: Create a working Rust project foundation.

Tasks:
1. Initialize Cargo workspace.
2. Add client crate.
3. Add server crate.
4. Add domain crate.
5. Add application crate.
6. Add infrastructure crate.
7. Add API contract crate.
8. Configure PostgreSQL.
9. Configure SQLx.
10. Add initial migration.
11. Add Axum health endpoint.
12. Add Dioxus app shell.
13. Configure GitHub Actions.

Sprint acceptance:
- Workspace compiles.
- Server starts.
- Health endpoint returns success.
- Client starts on desktop.
- Database migration runs.
- CI passes.

## 12. Suggested Repository Structure

```text
teamspace/
├── apps/
│   ├── client/
│   └── server/
├── crates/
│   ├── domain/
│   ├── application/
│   ├── infrastructure/
│   ├── api-contract/
│   ├── auth/
│   ├── realtime/
│   ├── ui-kit/
│   └── config/
├── migrations/
├── assets/
├── tests/
├── docs/
│   ├── PRD.md
│   ├── ARCHITECTURE.md
│   ├── DATABASE.md
│   └── API.md
├── Cargo.toml
├── rust-toolchain.toml
└── README.md
```

## 13. Initial Database Entities

- users
- oauth_accounts
- sessions
- projects
- project_members
- invitations
- tasks
- task_assignees
- checklist_items
- task_comments
- deliverables
- reviews
- notifications
- activity_events

Thiết kế schema chi tiết cần được hoàn thiện trong tài liệu DATABASE.md trước khi triển khai migration chính thức.

## 14. Product Success Metrics

Initial validation targets:

- 3–5 beta teams.
- 3–10 members per team.
- At least 80% of invited teams create a project.
- At least 3 teams complete one workflow.
- At least 2 teams remain active after four weeks.

Core metrics:
- Weekly Active Teams.
- Weekly Active Members.
- Tasks Completed.
- Project Completion Rate.
- Deliverable Review Completion.
- 4-week Team Retention.

Các mục tiêu trên là giả thuyết để kiểm chứng, không phải benchmark thị trường.

---

# PART IV — PRIORITY RULES

## 15. Priority Definition

**P0:** Cần thiết để hoàn thành workflow cốt lõi.

**P1:** Cải thiện trải nghiệm MVP sau khi P0 hoạt động.

**P2:** Mở rộng sản phẩm khi đã có dữ liệu sử dụng.

## 16. Final Product Decision

TeamSpace sẽ được xây dựng theo nguyên tắc:

1. Student-first.
2. Project Workflow là USP.
3. Project là trung tâm.
4. Task Discussion thay thế việc phụ thuộc hoàn toàn vào chat chung.
5. Deliverables và Review là điểm khác biệt.
6. Rust-first architecture.
7. Desktop và Mobile.
8. Phát triển incremental.
9. Ưu tiên kiểm chứng workflow trước khi mở rộng tính năng.

**MVP success condition:** Một nhóm sinh viên có thể tạo dự án, phân công nhiệm vụ, trao đổi, nộp sản phẩm và nhận phê duyệt mà không phải rời TeamSpace để quản lý trạng thái công việc.