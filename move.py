import os
import shutil
import exifread
from datetime import datetime
from tqdm import tqdm

# ---------------- 配置 ----------------
src_folder = r"X:\DCIM\100MEDIA"      # 源目录（示例路径，请改成你自己的）
dst_folder = r"C:\Photos\RAW"         # 目标目录（示例路径，请改成你自己的）
extensions = (".jpg", ".jpeg", ".nef", ".mp4", ".mov")  # 支持格式
# -------------------------------------

os.makedirs(dst_folder, exist_ok=True)

def get_file_date(file_path, ext):
    """获取照片或视频的拍摄日期"""
    ext = ext.lower()
    if ext in (".jpg", ".jpeg", ".nef"):  # 照片用 EXIF
        try:
            with open(file_path, "rb") as f:
                tags = exifread.process_file(f, stop_tag="EXIF DateTimeOriginal", details=False)
                if "EXIF DateTimeOriginal" in tags:
                    dt_str = str(tags["EXIF DateTimeOriginal"])
                    return datetime.strptime(dt_str, "%Y:%m:%d %H:%M:%S")
        except Exception:
            pass
    # 视频或无 EXIF → 用文件修改时间
    ts = os.path.getmtime(file_path)
    return datetime.fromtimestamp(ts)

def main():
    # 收集要处理的文件
    files = []
    for filename in os.listdir(src_folder):
        if filename.startswith("._"):  # 跳过 Mac 隐藏文件
            continue
        file_path = os.path.join(src_folder, filename)
        if os.path.isfile(file_path) and os.path.splitext(filename)[1].lower() in extensions:
            files.append(filename)

    # 进度条
    with tqdm(total=len(files), desc="整体进度", unit="个", ncols=80, leave=True) as pbar:
        for filename in files:
            file_path = os.path.join(src_folder, filename)
            name, ext = os.path.splitext(filename)

            dt = get_file_date(file_path, ext)
            folder_name = dt.strftime("%m_%d") if dt else "unknown"

            target_folder = os.path.join(dst_folder, folder_name)
            os.makedirs(target_folder, exist_ok=True)

            dst_path = os.path.join(target_folder, filename)

            # 避免重名覆盖
            counter = 1
            base, extension = os.path.splitext(dst_path)
            while os.path.exists(dst_path):
                dst_path = f"{base}_{counter}{extension}"
                counter += 1

            shutil.move(file_path, dst_path)

            # 同步移动 darktable 生成的 xmp 侧边栏文件
            xmp_src = file_path + ".xmp"
            if os.path.isfile(xmp_src):
                xmp_dst = dst_path + ".xmp"
                shutil.move(xmp_src, xmp_dst)
                tqdm.write(f"已移动: {xmp_src} -> {xmp_dst}")

            # 打印移动信息，不干扰进度条
            tqdm.write(f"已移动: {file_path} -> {dst_path}")
            pbar.update(1)

def fix_orphaned_xmp():
    """补迁：把源目录中遗留的 xmp 移动到目标目录对应照片旁边"""
    count = 0
    for root, dirs, files in os.walk(dst_folder):
        for f in files:
            if os.path.splitext(f)[1].lower() not in extensions:
                continue
            xmp_name = f + ".xmp"
            xmp_src = os.path.join(src_folder, xmp_name)
            if os.path.isfile(xmp_src):
                xmp_dst = os.path.join(root, xmp_name)
                shutil.move(xmp_src, xmp_dst)
                print(f"已补迁: {xmp_src} -> {xmp_dst}")
                count += 1
    print(f"完成，共补迁 {count} 个 xmp 文件")

if __name__ == "__main__":
    main()
    fix_orphaned_xmp()
