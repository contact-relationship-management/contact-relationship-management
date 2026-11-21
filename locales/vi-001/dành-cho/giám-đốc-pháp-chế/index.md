# Dành cho Giám đốc pháp chế

> Bản tóm tắt kỹ thuật, không phải tư vấn pháp lý, chứng nhận hay kiểm toán. Bản này nêu
> những gì kho mã cho thấy tính đến 2026-10-11. Không nội dung nào ở đây tuyên bố được công nhận.

## Tóm tắt lập trường pháp lý

Kho mã **chưa sẵn sàng cho môi trường sản xuất** và chứa dữ liệu tổng hợp. Nó không tuyên bố tuân thủ bất kỳ chế độ nào.
Nó triển khai các tính năng kỹ thuật hỗ trợ một số chế độ, và liệt kê những gì bên triển khai phải bổ sung
([regulatory](../../../../spec/regulatory.md)).

## Các chế độ liên quan

| Chế độ | Phần mềm hỗ trợ gì | Bạn phải cung cấp gì |
|---|---|---|
| GDPR của Anh/EU | Lịch sử đồng ý, xuất dữ liệu để truy cập dữ liệu cá nhân, xóa (bị từ chối khi còn nghĩa vụ mở), quét lưu giữ, tối thiểu hóa dữ liệu, kiểm toán các lần đọc | Cơ sở pháp lý cho từng mục đích, DPIA ([mẫu](../../../../spec/compliance/dpia-template.md)), hồ sơ xử lý, lịch lưu giữ, vai trò bên kiểm soát/bên xử lý |
| PECR / e-privacy | Gửi chỉ khi có đồng ý, hủy đăng ký ngay lập tức | Nhận diện người gửi và các quyết định về soft opt-in cho một người gửi thật |
| HIPAA (nếu là dữ liệu y tế của Mỹ) | Các tính năng kiểm soát kiểm toán, kiểm soát truy cập, toàn vẹn, an toàn truyền tải | Thỏa thuận đối tác kinh doanh, phân tích rủi ro, chính sách, đào tạo nhân sự. Phần mềm không làm bạn tuân thủ |
| NHS DTAC / DCB0129/0160 | Bản nháp bằng chứng ([dtac-supporting](../../../../spec/compliance/dtac-supporting.md)) | Cán bộ an toàn lâm sàng được chỉ định, sổ nhật ký mối nguy, các chứng nhận. Đánh giá cho biết DTAC có khả năng không áp dụng |
| Khả năng truy cập (WCAG 2.2 AA, khu vực công) | Kiểm tra tự động và một tuyên bố trung thực | Một cuộc kiểm toán thủ công (T48) trước mọi tuyên bố tuân thủ |
| An ninh chính phủ (NIST 800-53, ISO 27001, Cyber Essentials) | Ánh xạ biện pháp kiểm soát ([control-mapping](../../../../spec/compliance/control-mapping.md)) | Chứng nhận và bảo đảm độc lập |

## Các hợp đồng bạn sẽ cần mà chưa tồn tại

1. Thỏa thuận xử lý dữ liệu và điều khoản chuyển dữ liệu quốc tế với nhà cung cấp email (T39).
2. Bản mô tả công việc và điều khoản bảo mật với công ty kiểm thử xâm nhập (T42).
3. Điều khoản lưu trữ và bên xử lý phụ cho PostgreSQL, Redis và nhật ký.
4. Giấy phép: xem [LICENSE.md](../../../../LICENSE.md); kho mã cũng kèm các tuyên bố về
   bảo mật, quản trị và việc sử dụng AI.

## Những điểm pháp lý cần theo dõi

- **Xóa** ẩn danh hóa liên hệ và bị từ chối khi còn giao dịch mở, phiếu hỗ trợ hoặc đăng ký chuỗi nuôi dưỡng đang
  hoạt động; chuỗi kiểm toán giữ lại hồ sơ các hành động, vì vậy hãy xác định trong DPIA của bạn rằng điều đó tương
  tác thế nào với các yêu cầu xóa.
- **Break-glass**: việc đọc dữ liệu bị che chỉ hợp pháp trên cơ sở được ghi nhận của bạn; trường lý do là bằng chứng,
  không phải thẩm quyền.
- **Chấm điểm** khách hàng tiềm năng: nếu nó từng ảnh hưởng đến các quyết định quan trọng về con người, các quy tắc về
  quyết định tự động sẽ áp dụng ([regulatory](../../../../spec/regulatory.md)).
- **Cam kết về API:** [api-lifecycle](../../../../spec/compliance/api-lifecycle.md)
  đặt thời hạn thông báo 180 ngày trước khi một thay đổi API bị gỡ bỏ. Đó là lựa chọn của người bảo trì; nếu khách hàng
  sẽ dựa vào đó, hãy quyết định liệu nó có nên nằm trong hợp đồng.
- **Tuyên bố công khai:** đừng nêu việc tuân thủ, chứng nhận hay phê duyệt.
