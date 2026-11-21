# Lộ trình lên sản xuất

> Contact Relationship Management chưa sẵn sàng cho môi trường sản xuất. Trang này nói rõ môi trường sản xuất có nghĩa
> là gì ở đây, và mỗi phần đã đi được đến đâu.

Phần mềm chạy trên dữ liệu tổng hợp. Email được mô phỏng trừ khi cấu hình một chế độ thư thật. Đừng đưa dữ liệu cá nhân
thật vào đó.

## Môi trường sản xuất nghĩa là gì

Một bản triển khai sản xuất giữ dữ liệu của người thật và gửi thông điệp thật cho họ. Với dự án này, điều đó cần tất cả
những điều sau:

- **Đăng nhập thật.** Mọi yêu cầu đều được xác thực, và các quy tắc truy cập khớp với các vai trò của tổ chức triển khai.
- **Gửi hợp pháp.** Mỗi thông điệp có sự đồng ý được ghi nhận và cơ sở pháp lý theo luật của mọi nơi mà nó được gửi
  đến, và đi qua một nhà cung cấp email thật xử lý thư trả lại, khiếu nại và hủy đăng ký.
- **Bảo đảm độc lập.** Những người ngoài dự án đã kiểm thử bảo mật và khả năng truy cập, và các phát hiện đã được sửa
  hoặc được chấp nhận.
- **Vận hành.** Các bản sao lưu đã được khôi phục thử, giám sát, quy trình xử lý sự cố và người trực.
- **Bằng chứng về độ ổn định.** Chạy dài hạn dưới tải thực tế, trên phần cứng giống bản triển khai, không có lỗi hay
  chậm đi.

Mức sẵn sàng cho sản xuất được quyết định theo từng bản triển khai. Bên triển khai chỉ có thể đóng một số cổng cho tổ
chức và người nhận của riêng mình. Đánh giá kỹ thuật làm nền cho trang này là [spec/plan.md](../../../spec/plan.md). Các
cổng còn mở nằm trong [spec/tasks.md](../../../spec/tasks.md).

## Mức sẵn sàng

Mỗi tỷ lệ phần trăm là ước tính được thực hiện vào 2026-11-13 từ danh sách tác vụ, không phải một phép đo.

- **Xác thực và kiểm soát truy cập** (70%): Đã xây dựng việc đăng nhập bằng token PASETO hoặc Keycloak/OIDC, quy tắc
  truy cập dựa trên thuộc tính với kiểm tra ở cấp bản ghi, và một chế độ sản xuất từ chối khởi động nếu không có xác
  thực. Chưa có bản triển khai thật nào bật chúng với các thuộc tính của riêng mình, và ma trận thực thi chưa bao phủ
  mọi trình xử lý (CRM-G1).
- **Gửi email** (40%): Mặc định là bộ gửi mô phỏng, và có một bộ gửi SMTP. Chưa có bộ điều hợp nhà cung cấp email với
  webhook cho thư trả lại và khiếu nại hay danh sách chặn (CRM-T39).
- **Bảo vệ dữ liệu và luật tiếp thị điện tử** (15%): Đã xây dựng hồ sơ đồng ý, truy cập dữ liệu cá nhân, xóa và lưu
  giữ. Chưa đọc luật của vùng nào cho dự án này, và việc rà soát đường gửi thật còn mở (CRM-G2,
  [regulatory](../../../spec/regulatory.md)).
- **Bảo đảm bảo mật độc lập** (10%): Đã viết phạm vi kiểm thử xâm nhập. Chưa có kiểm thử xâm nhập, phân tích rủi ro
  chính thức hay chứng nhận nào (CRM-T42). Các cuộc kiểm toán bảo mật bằng AI không phải là bảo đảm độc lập.
- **Khả năng truy cập** (50%): Các kiểm tra tự động axe-core theo WCAG 2.2 A và AA đạt trên mọi tuyến trang. Chưa có
  cuộc kiểm toán thủ công nào với việc chỉ dùng bàn phím, trình đọc màn hình hay phóng to 400% (CRM-T48).
- **Bản dịch** (30%): Giao diện có sẵn bằng tám ngôn ngữ. Các chuỗi bằng ngôn ngữ khác ngoài tiếng Anh do máy viết, và
  chưa có người bản ngữ nào xem lại.
- **Vận hành** (30%): Một sổ tay vận hành bao gồm sao lưu và khôi phục, quản lý khóa, mã hóa khi lưu trữ, xử lý sự cố
  và lưu giữ nhật ký. Chưa có lần khôi phục nào được kiểm thử, và danh sách kiểm tra trước khi đưa vào hoạt động của sổ
  tay còn mở.
- **Hiệu năng và độ ổn định** (20%): Đã có kiểm thử tải và kiểm thử ổn định chạy lâu. Kiểm thử ổn định đã chạy hai
  phút trên hệ thống demo; chưa có lần chạy dài và chưa có mức cơ sở trên phần cứng giống môi trường sản xuất.
- **Tính toàn vẹn của bản phát hành** (85%): Các image phát hành hỗ trợ nhiều kiến trúc, được ký và chứng thực, và chữ
  ký của chúng xác minh được. Các action và image được ghim theo digest, và lịch sử được quét tìm bí mật. Nửa amd64 của
  một image phát hành đã tải về chưa được kiểm tra, và chưa có máy chủ mã thứ hai (CRM-T192).
- **Tích hợp liên tục** (100%): Một tập lệnh chạy mọi kiểm tra trên GitHub Actions và Woodpecker. Cả hai codebase Rust
  và giao diện đều có ngưỡng tối thiểu 95% số dòng; có các cổng trình duyệt, lint, chuỗi cung ứng, đột biến trên các
  dòng thay đổi và đột biến toàn bộ hằng tuần (CRM-T94).
- **Biện pháp bảo vệ bản demo** (100%): Mỗi khách tham quan nhận một bản sao riêng của dữ liệu demo, tối đa 200 bản
  sao. Phần giới thiệu mặc định tắt; đặt `CONTACT_RELATIONSHIP_MANAGEMENT_DEMO_TOUR=on` để bật (CRM-T92).

## Cách giữ trang này cập nhật

Khi một cổng đóng lại, hãy đổi ước tính và bằng chứng của nó trong cùng commit với công việc, và liên kết tác vụ.
