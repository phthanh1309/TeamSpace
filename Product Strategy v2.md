# TeamSpace — Product Strategy v2

## 1. Product Positioning

### Định vị sản phẩm

**TeamSpace — Project-first Collaboration for Student Teams**

TeamSpace là nền tảng cộng tác dành cho các nhóm học tập, giúp thành viên tổ chức dự án, phân chia nhiệm vụ, trao đổi, quản lý tài liệu và hoàn thành sản phẩm chung trong một không gian thống nhất.

### Vấn đề cần giải quyết

Khi thực hiện một bài thuyết trình, bài tập nhóm hoặc dự án nghiên cứu, các nhóm thường gặp những vấn đề:

- Công việc được phân chia qua tin nhắn nhưng khó theo dõi tiến độ.
- Deadline và trách nhiệm của từng thành viên không rõ ràng.
- Tài liệu nằm rải rác trên Google Drive, Canva và các nền tảng khác.
- Feedback bị trôi trong các cuộc trò chuyện.
- Khó xác định phiên bản nào là sản phẩm cuối cùng.
- Người quản lý nhóm khó biết dự án đang gặp vấn đề ở đâu.

### Giải pháp của TeamSpace

Đưa toàn bộ quy trình làm việc của một dự án vào cùng một hệ thống:

**Project → Tasks → Discussion → Deliverables → Review → Completion**

Mỗi nhiệm vụ không chỉ có người phụ trách và deadline mà còn có thể gắn với thảo luận, tài liệu tham khảo, sản phẩm bàn giao và quá trình đánh giá.

---

## 2. USP — Unique Selling Proposition

### USP chính

> TeamSpace giúp nhóm học tập hoàn thành dự án cùng nhau bằng cách kết nối nhiệm vụ, thảo luận, tài liệu và sản phẩm bàn giao trong một workflow thống nhất.

### Ba trụ cột khác biệt

#### 2.1. Project-centered workflow

Dự án là trung tâm của hệ thống, không phải cuộc trò chuyện.

- Mỗi dự án có thành viên, nhiệm vụ và tiến độ riêng.
- Mỗi nhiệm vụ có người phụ trách, deadline và checklist.
- Thảo luận có thể gắn trực tiếp với nhiệm vụ.
- Deadline và trạng thái công việc được tổng hợp tại Project Overview.

#### 2.2. Deliverable management

TeamSpace theo dõi kết quả thực tế của công việc, không chỉ theo dõi trạng thái task.

Một nhiệm vụ có thể chứa:
- Link Canva.
- Link Google Drive.
- File PDF, PPTX, DOCX và hình ảnh.
- Ghi chú và tài liệu tham khảo.
- Sản phẩm được gửi để review.
- Feedback và trạng thái phê duyệt.

#### 2.3. Review workflow

Hỗ trợ nhóm thực hiện quy trình:

**Assigned → In Progress → Submitted → In Review → Approved**

Người phụ trách gửi sản phẩm, thành viên khác phản hồi và người có quyền review xác nhận kết quả.

Đây là hướng phát triển tạo khác biệt rõ ràng hơn so với việc chỉ cung cấp chat và danh sách công việc.

---

## 3. Target Users

### Thị trường chính: Student Teams

Đối tượng ban đầu:

- Học sinh THPT.
- Sinh viên đại học, cao đẳng.
- Nhóm làm bài tập và thuyết trình.
- Nhóm nghiên cứu hoặc thực hiện dự án học tập.
- Câu lạc bộ học thuật và tổ chức sự kiện trong trường.

### Nhóm người dùng ưu tiên

**Primary users:**
- Nhóm 3–10 thành viên.
- Có ít nhất một dự án chung.
- Thường xuyên phân chia nhiệm vụ.
- Có deadline và sản phẩm cần bàn giao.

**Secondary users:**
- Nhóm thiết kế, sáng tạo nội dung.
- Nhóm tổ chức sự kiện.
- Nhóm phát triển phần mềm nhỏ.

### User personas

**1. Project Leader**

- Tạo dự án và mời thành viên.
- Chia nhiệm vụ và phân công.
- Theo dõi deadline.
- Kiểm tra tiến độ và duyệt sản phẩm.

**2. Team Member**

- Xem nhiệm vụ được giao.
- Cập nhật tiến độ.
- Trao đổi với nhóm.
- Nộp sản phẩm và nhận feedback.

**3. Reviewer**

- Kiểm tra sản phẩm.
- Gửi nhận xét.
- Yêu cầu chỉnh sửa hoặc phê duyệt.

Một người có thể đảm nhiệm nhiều vai trò trong cùng dự án.

---

## 4. MVP Scope

Mục tiêu MVP: giúp một nhóm học tập tạo dự án, chia việc, phối hợp và hoàn thành ít nhất một sản phẩm chung.

### P0 — Bắt buộc có trong MVP

| Module | Chức năng |
|---|---|
| Authentication | Đăng nhập Google, Facebook; quản lý tài khoản |
| Projects | Tạo dự án, mời thành viên, chỉnh sửa thông tin |
| Tasks | Tạo task, giao việc, deadline, trạng thái, checklist |
| Task Discussion | Bình luận và trao đổi theo từng task |
| Deliverables | Đính kèm file hoặc liên kết sản phẩm |
| Review | Gửi sản phẩm, feedback, yêu cầu chỉnh sửa, duyệt |
| Home | Tổng quan dự án, task của tôi, deadline sắp tới |
| Notifications | Thông báo được giao việc, bình luận và review |
| Settings | Ngôn ngữ, giao diện, thông báo và tài khoản |

### P1 — Sau khi MVP được kiểm chứng

- Kanban board.
- Project activity timeline.
- Tin nhắn trực tiếp và nhóm.
- Tìm kiếm trong dự án.
- Phiên bản sản phẩm bàn giao.
- Mẫu dự án học tập.
- Tài liệu và ghi chú dùng chung.

### P2 — Phát triển dài hạn

- Realtime presence.
- Phân tích hiệu suất dự án.
- Workflow tùy chỉnh.
- Tích hợp sâu với Google Drive và Canva.
- Tự động hóa công việc.
- AI hỗ trợ tóm tắt và quản lý dự án.

### Những gì không nên làm ngay

- Không xây dựng hệ thống chat phức tạp như Slack.
- Không làm trình chỉnh sửa tài liệu hoặc slide.
- Không xây dựng Calendar độc lập.
- Không làm hệ thống analytics lớn.
- Không triển khai microservices.
- Không tích hợp quá nhiều dịch vụ bên ngoài ngay từ đầu.

---

## 5. Product Experience

### Desktop Navigation

1. Home
2. Messages
3. Projects
4. Documents
5. Profile
6. Settings

Projects là khu vực quan trọng nhất.

### Mobile Navigation

- Home
- Messages
- Projects
- Documents
- Profile

Settings nằm trong Profile.

### Home Dashboard

Hiển thị:
- Dự án gần đây.
- Công việc được giao.
- Deadline hôm nay, quá hạn và trong 7 ngày tới.
- Hoạt động gần đây.
- Tiến độ dự án.

### Project Workspace

Mỗi dự án có:

- Overview.
- Tasks.
- Discussion.
- Deliverables.
- Members.
- Activity.

Không cần biến từng mục thành một ứng dụng riêng. Chúng phải kết nối với nhau trong cùng một workflow.

### Ví dụ workflow: School Presentation

1. Leader tạo dự án "Biến đổi khí hậu".
2. Mời 5 thành viên.
3. Tạo các task nghiên cứu, viết nội dung và thiết kế slide.
4. Giao nhiệm vụ và deadline.
5. Thành viên trao đổi ngay trong task.
6. Người thiết kế gửi link Canva.
7. Leader review và yêu cầu chỉnh sửa.
8. Thành viên cập nhật sản phẩm.
9. Leader phê duyệt.
10. Dự án hoàn thành.

Đây là workflow mẫu cần được ưu tiên kiểm chứng trước khi mở rộng sản phẩm.

---

## 6. Roadmap

### Phase 0 — Product Validation

**Mục tiêu:** Xác nhận vấn đề thực sự tồn tại.

- Phỏng vấn 5–10 nhóm học tập.
- Tìm hiểu cách họ đang phân chia nhiệm vụ.
- Xác định công cụ đang sử dụng.
- Tìm hiểu vấn đề lớn nhất khi làm bài tập nhóm.
- Kiểm tra prototype với người dùng.

Deliverable:
- Product requirement document.
- User journey.
- MVP wireframe.
- Danh sách vấn đề được xác nhận.

### Phase 1 — Foundation

**Mục tiêu:** Xây dựng nền tảng kỹ thuật.

- Rust workspace.
- Dioxus client cho Desktop và Mobile.
- Axum backend.
- PostgreSQL và SQLx.
- Authentication.
- User và project membership.
- Database migrations.
- API contracts.
- CI và kiểm thử cơ bản.

### Phase 2 — Core Project Workflow

**Mục tiêu:** Hoàn thành quy trình làm việc cốt lõi.

- Tạo và quản lý dự án.
- Mời thành viên.
- Tạo, giao và cập nhật task.
- Deadline và checklist.
- Task discussion.
- Home dashboard.
- Project overview.

Đây là phase quan trọng nhất của MVP.

### Phase 3 — Deliverables & Review

**Mục tiêu:** Hoàn thiện điểm khác biệt sản phẩm.

- Upload file và quản lý metadata.
- Gắn link Canva/Drive.
- Gửi sản phẩm bàn giao.
- Feedback và review.
- Trạng thái approved / revision requested.
- Lưu lịch sử hoạt động cơ bản.

### Phase 4 — Beta Release

**Mục tiêu:** Đưa sản phẩm vào sử dụng thực tế.

- Mời 3–5 nhóm học tập.
- Theo dõi cách họ sử dụng sản phẩm.
- Phỏng vấn người dùng sau khi hoàn thành dự án.
- Sửa lỗi và cải thiện UX.
- Đo mức độ quay lại sử dụng.

### Phase 5 — Growth & Expansion

**Mục tiêu:** Mở rộng sau khi workflow cốt lõi hoạt động tốt.

- Template dự án.
- Kanban.
- Direct messaging và group messaging.
- Documents.
- Search.
- Tích hợp dịch vụ bên ngoài.
- Analytics và các tính năng nâng cao.

---

## 7. Go-to-Market Strategy

### Giai đoạn đầu: Beta trong cộng đồng học tập

Tìm 3–5 nhóm học tập có dự án thực tế để thử nghiệm.

Ưu tiên:
- Nhóm làm bài thuyết trình.
- Nhóm nghiên cứu.
- Nhóm tổ chức sự kiện.
- Nhóm làm dự án học thuật.

### Product-led acquisition

Tạo các template miễn phí:

- School Presentation.
- Group Assignment.
- Research Project.
- Event Planning.
- Design Project.

Người dùng có thể tạo dự án từ template thay vì bắt đầu từ trang trống.

### Nội dung giới thiệu sản phẩm

Tập trung trình diễn một quy trình thực tế:

"Nhóm 5 người hoàn thành một bài thuyết trình bằng TeamSpace."

Không chỉ giới thiệu danh sách tính năng.

### Thông điệp marketing

**Stop chasing messages. Start completing projects.**

Thông điệp tiếng Việt:

**Đừng để công việc nhóm bị mắc kẹt trong những cuộc trò chuyện. Hãy cùng nhau hoàn thành dự án.**

---

## 8. Success Metrics

Các con số dưới đây là mục tiêu thử nghiệm ban đầu, không phải benchmark thị trường.

| Metric | Mục tiêu thử nghiệm |
|---|---:|
| Nhóm beta | 3–5 |
| Thành viên mỗi nhóm | 3–10 |
| Nhóm tạo được dự án | ≥ 80% |
| Nhóm hoàn thành ít nhất một workflow | ≥ 3 |
| Nhóm còn hoạt động sau 4 tuần | ≥ 2 |

Các chỉ số cần theo dõi:

- Weekly Active Teams.
- Weekly Active Members.
- Tasks Completed.
- Project Completion Rate.
- 4-week Team Retention.
- Deliverable Review Completion.

Chỉ nên mở rộng tính năng khi dữ liệu sử dụng cho thấy workflow hiện tại thực sự hữu ích.

---

## 9. Product Principles

1. Project-first: dự án là trung tâm.
2. Workflow over feature count: ưu tiên quy trình hoàn chỉnh thay vì số lượng tính năng.
3. Simple by default: dễ dùng với người không có kinh nghiệm quản lý dự án.
4. Deliverable-oriented: tập trung vào kết quả cuối cùng.
5. Cross-platform: Desktop và Mobile.
6. Rust-first architecture: Dioxus, Axum, Tokio, SQLx.
7. Free-first infrastructure: ưu tiên giải pháp miễn phí hoặc chi phí thấp trong giai đoạn đầu.
8. Incremental delivery: phát triển từng phase, không xây dựng toàn bộ hệ thống ngay lập tức.

---

## 10. Strategic Summary

| Thành phần | Quyết định |
|---|---|
| Product | TeamSpace |
| Market | Student-first |
| Primary audience | Student teams |
| Main differentiator | Project Workflow |
| Product model | Project-first Collaboration |
| Core value | Hoàn thành dự án nhóm trong một workflow thống nhất |
| MVP focus | Project, Tasks, Discussion, Deliverables, Review |
| Platforms | Desktop + Mobile |
| UI | Dioxus |
| Backend | Rust + Axum + Tokio |
| Database | PostgreSQL + SQLx |
| Infrastructure | Free-first, cloud-first |
| Initial acquisition | Student communities and beta teams |

**Chiến lược cốt lõi:** TeamSpace không cần có nhiều tính năng hơn các nền tảng lớn. TeamSpace cần làm tốt hơn một việc cụ thể: giúp các nhóm học tập phối hợp và hoàn thành dự án mà không bị phân tán giữa nhiều công cụ.