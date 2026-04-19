fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let ico_path = format!("{out_dir}/icon.ico");

    let png_bytes = include_bytes!("Assets/Icons/icon.png");
    let img = image::load_from_memory(png_bytes).expect("failed to decode icon PNG");

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [256u32, 128, 64, 48, 32, 16] {
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let rgba = resized.into_rgba8();
        let icon_image = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());
        icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image).unwrap());
    }

    let mut file = std::fs::File::create(&ico_path).unwrap();
    icon_dir.write(&mut file).unwrap();

    let mut res = winres::WindowsResource::new();
    res.set_icon(&ico_path);
    res.compile().unwrap();
}
