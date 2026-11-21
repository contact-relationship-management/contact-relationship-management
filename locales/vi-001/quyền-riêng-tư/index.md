# Thông báo quyền riêng tư: những gì được lưu trong trình duyệt của bạn

Thông báo này bao gồm hai phần: trang web này (`contact-relationship-management.github.io`) và giao diện người dùng
Contact Relationship Management như kho mã này cung cấp. Thông báo liệt kê mọi thứ mà mỗi phần lưu trong trình duyệt
của bạn và lý do.

Một tổ chức vận hành CRM sẽ xử lý dữ liệu cá nhân về các liên hệ của mình. Việc xử lý đó thuộc về thông báo quyền
riêng tư của chính tổ chức đó, không thuộc thông báo này ([spec/regulatory.md](../../../spec/regulatory.md)).

## Những gì được lưu, và vì sao

Mỗi mục lưu một lựa chọn mà bạn đã đưa ra: giao diện, ngôn ngữ và vùng, cỡ chữ, múi giờ hoặc bố cục của một
bảng. Không mục nào lưu tên, địa chỉ email, địa chỉ IP hay bất kỳ mã định danh nào của bạn, và không mục nào được dùng
để theo dõi bạn, đo lường bạn hay hiển thị quảng cáo cho bạn. Mỗi mục tồn tại để trang tiếp theo bạn mở dùng cùng
lựa chọn đó, và bạn không phải chọn lại.

Không có gì được lưu cho đến khi bạn đưa ra một lựa chọn. Một khách truy cập không thay đổi gì sẽ không có gì được lưu.

### Trên trang web này

| Tên | Loại | Nội dung lưu | Thời gian lưu |
| --- | --- | --- | --- |
| `lily-theme` | Cookie và `localStorage` | `system` ("Theo thiết bị này") hoặc tên của một giao diện, chẳng hạn `dark` | Cookie: 400 ngày. `localStorage`: cho đến khi bạn xóa |
| `lily-text-size` | `localStorage` | Cỡ chữ bạn đã chọn | Cho đến khi bạn xóa |
| `lily-time-zone` | `localStorage` | Múi giờ bạn đã chọn | Cho đến khi bạn xóa |

Giao diện được lưu trong cookie và trong `localStorage`, để vẫn còn nếu trình duyệt của bạn xóa một trong hai.
Các trang của trang web là tệp tĩnh, và không có chương trình nào trên trang web đọc cookie. Vì là cookie, trình duyệt
của bạn gửi nó đến GitHub Pages, nơi lưu trữ trang web, cùng với mỗi yêu cầu đến trang web.

Ngôn ngữ và vùng của trang web nằm trong địa chỉ trang, chẳng hạn `/de-de/`, nên không có gì được lưu cho chúng.

### Trong giao diện người dùng CRM

| Tên | Loại | Nội dung lưu | Thời gian lưu |
| --- | --- | --- | --- |
| `crm-theme` | Cookie | `system` ("Theo thiết bị này") hoặc tên của một giao diện | 400 ngày |
| `mxi.crm.locale` | `localStorage` | Ngôn ngữ và vùng bạn đã chọn, chẳng hạn `de-de` | Cho đến khi bạn xóa |
| `lily-text-size` | `localStorage` | Cỡ chữ bạn đã chọn | Cho đến khi bạn xóa |
| `crm-grid-<table>` | `localStorage` | Độ rộng cột, cột bị ẩn và thứ tự sắp xếp của một bảng | Cho đến khi bạn xóa |
| `crm-grid-order-<table>` | `localStorage` | Thứ tự cột của một bảng | Cho đến khi bạn xóa |
| `__Host-mxi_session` | Cookie | Một mã ngẫu nhiên cho phiên đăng nhập của bạn | Cho đến khi bạn đăng xuất hoặc phiên kết thúc |

Máy chủ của CRM đọc cookie `crm-theme` để gửi mỗi trang với giao diện của bạn, nhờ đó trang không xuất hiện
trước tiên ở một giao diện khác. CRM không bao giờ đọc nó cho mục đích nào khác.

Cookie phiên không phải là một lựa chọn. Nó được đặt khi bạn đăng nhập, các tập lệnh của trình duyệt không đọc được nó
(`HttpOnly`), và nếu không có nó bạn không thể duy trì đăng nhập.

## Ai nhận được

Các mục này ở lại trong trình duyệt của bạn. Không có gì trong đó được gửi cho bên thứ ba, ngoại trừ cookie giao diện
của trang web, mà trình duyệt của bạn gửi đến máy chủ của trang như mô tả ở trên. Các cookie là của bên thứ nhất
(`SameSite=Lax`), và là `Secure` trên một trang `https`.

## Cách xóa

Để thay đổi một lựa chọn, hãy chọn lại. Để xóa mọi thứ, hãy xóa dữ liệu của trang web này trong phần cài đặt của
trình duyệt: khi đó trang tiếp theo lại dùng các giá trị mặc định, nghĩa là theo thiết lập sáng hoặc tối của thiết bị,
cỡ chữ tiêu chuẩn và ngôn ngữ mặc định của trang.
