# Dành cho Giám đốc thông tin

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Góc nhìn về thông tin và dữ liệu

CRM này lưu **dữ liệu quan hệ**: các liên hệ và tài khoản dưới dạng lớp bao quanh các mã định danh được lưu ở hệ thống
khác, cùng hoạt động, giao dịch, phiếu hỗ trợ, lịch sử đồng ý và nhật ký kiểm toán. Dữ liệu nhân khẩu học được cố ý giữ
lại trong các sổ đăng ký định danh ([domain-model](../../../../spec/domain-model.md)). Nếu tổ chức của bạn ghi nhận bệnh
nhân hay công dân là liên hệ, việc phân loại dữ liệu đó là do **bạn** quyết định; kế hoạch được viết theo cách hiểu
nghiêm ngặt hơn ([plan](../../../../spec/plan.md)).

## Dữ liệu đi đâu

| Nơi lưu | Chứa gì | Biện pháp bảo vệ trong kho mã |
|---|---|---|
| PostgreSQL | Mọi bản ghi, chuỗi kiểm toán, các quyền break-glass | Mã hóa trường ở hai cột; trigger kiểm toán chỉ ghi thêm; mã hóa khi lưu trữ thuộc về bên triển khai |
| Redis | Bộ nhớ đệm phản hồi insight (30 giây), bộ đếm giới hạn tốc độ, khóa quyền break-glass | Khóa theo từng người gọi; **cần TLS, mật khẩu và không được mở công khai** |
| Nhật ký/số liệu | Siêu dữ liệu yêu cầu, số liệu Prometheus | Thời gian lưu giữ thuộc về bên triển khai |
| Nhà cung cấp email (tương lai) | Nội dung thông điệp và người nhận | Chưa xây dựng (T39) |

## Tích hợp và định danh

- Đăng nhập đến từ một dịch vụ định danh phát hành token PASETO, hoặc Keycloak. Vì vậy việc nhân sự vào/chuyển/rời, MFA
  và chính sách phiên là của **nhà cung cấp định danh của bạn**; CRM thực thi những gì token nói và từ chối các token đã
  hết hạn.
- API được ghi tài liệu bằng OpenAPI ([gds-api-review](../../../../spec/compliance/gds-api-review.md)), đánh phiên bản
  qua header, phân trang bằng header `Link`, và trả lỗi dưới dạng tài liệu vấn đề RFC 9457. Các quy tắc thay đổi và ngừng
  hỗ trợ nằm trong [api-lifecycle](../../../../spec/compliance/api-lifecycle.md).
- SBOM (CycloneDX) được tạo cho dịch vụ và giao diện trong CI; quy trình phát hành ký chúng và các image.

## Nghĩa vụ quản trị thông tin vẫn thuộc về bạn

Bằng chứng sao lưu và khôi phục, lịch lưu giữ theo từng bảng, neo đầu chuỗi kiểm toán bên ngoài cơ sở dữ liệu, quyền giữ
khóa trường trong KMS, thỏa thuận xử lý dữ liệu với mọi nhà cung cấp, và đánh giá tác động bảo vệ dữ liệu
([dpia-template](../../../../spec/compliance/dpia-template.md)). Bản ánh xạ biện pháp kiểm soát
([control-mapping](../../../../spec/compliance/control-mapping.md)) đánh dấu mỗi biện pháp là
Implemented, Deployer hoặc Gap.
