**Türkçe** | [简体中文](https://github.com/Seyud/device_faker/blob/main/docs/README.md) | [English](https://github.com/Seyud/device_faker/blob/main/docs/en/README.md)

# Device Faker 📱

<img src="../logo.png" style="width: 96px;" alt="logo">

Zygisk tabanlı, farklı uygulamalar için farklı cihaz modellerini yapılandırabilen bir cihaz modeli taklit modülü.

[![Version](https://img.shields.io/github/v/release/Seyud/Device_Faker?logo=github)](https://github.com/Seyud/Device_Faker/releases/latest)
[![GitHub Downloads](https://img.shields.io/github/downloads/Seyud/Device_Faker/total?logo=github&logoColor=green)](https://github.com/Seyud/Device_Faker/releases)
[![Language](https://img.shields.io/badge/language-Rust-orange?logo=rust&logoColor=orange)](https://www.rust-lang.org/)
[![Telegram](https://img.shields.io/badge/group-Telegram-2CA5E0?logo=telegram&logoColor=87CEEB)](https://t.me/device_faker)

## Özellikler ✨

* 🎯 **Hassas Kontrol**: Her uygulama için cihaz bilgilerini ayrı ayrı yapılandırın.
* 📁 **Şablon Yönetimi**: Birden fazla cihaz şablonu oluşturun ve bunları kolayca birden fazla paket adına uygulayın.
* 🔄 **Anında Etki**: Yapılandırmayı değiştirdikten sonra yalnızca uygulamayı yeniden başlatmanız yeterlidir, sistemi yeniden başlatmanıza gerek yoktur.
* 🛡️ **Güvenli ve Kararlı**: Zygisk altyapısı üzerine kurulmuştur ve modüler tasarıma sahiptir.
* 📝 **Basit Yapılandırma**: Düzenlemesi kolay TOML formatındaki yapılandırma dosyalarını kullanır.
* ⚡ **Performans Odaklı**: Yalnızca yapılandırılmış uygulamalarda etkili olur, diğer uygulamaları etkilemez.
* 🎭 **Üç Mod Desteği**:
    * **Lite Mod**: Hafif ve düşük iz bırakan sahteleştirme.
    * **Full Mod**: Kapsamlı cihaz sahteleştirme.
    * **Companion Modu**: Sistem özelliklerini ve CPU bilgisini taklit eder.
* 🌐 **WebUI Yönetimi**: Yapılandırmaları kolayca yönetebilmeniz için grafiksel bir web arayüzü sunar.


## WebUI Özellikleri 🖥️

Device Faker modern, web tabanlı bir yönetim arayüzü sunar.

- 📊 **Durum Görüntüleme**: Modülün çalışma durumunu gerçek zamanlı olarak görüntüleme imkanı sunar
- 📋 **Şablon Yönetimi**: Cihaz şablonları oluşturun, düzenleyin ve silin; birden fazla paket adına toplu olarak uygulayın
- 📱 **Uygulama Yönetimi**: Yüklü uygulamalar ve bunların yapılandırma durumuna ilişkin sezgisel bir görünüm
- 🖋️ **Yapılandırma Düzenleme**: Uygulama yapılandırmasını düzenlemek için grafik arayüz; şablon uygulamaları ve özel yapılandırmaları destekler

## Yapılandırma Rehberi ⚙️

Ayrıntılı yapılandırma talimatları için lütfen [Yapılandırma Belgeleri](https://github.com/Seyud/device_faker/blob/main/docs/en/CONFIG.md) bölümüne bakın.

> Bu kısım çok fazla teknik bilgi içerdiğinden dolayı çevirme gereği duymadım, zaten eğer yapılandırma dosyaları ile uğraşıyorsanız ortalama seviyede bir İngilizceye sahipsiniz demektir :)

Yapılandırma dosyası `/data/adb/device_faker/config/config.toml` konumunda bulunur ve TOML biçimini kullanır. Yapılandırmayı değiştirdikten sonra, değişikliklerin yürürlüğe girmesi için ilgili uygulamayı yeniden başlatmanız yeterlidir; sistemi yeniden başlatmanıza gerek yoktur.

## Şablon Yapılandırması Katkıları 🎁

Topluluğun katkıları için teşekkür ediyoruz, siz de depoya katkı sağlayabilirsiniz!

- 📦 [device_faker_config](https://github.com/Seyud/device_faker_config) - Cihaz şablonu yapılandırmalarına katkı sağlayın

Cihaz yapılandırmalarınızı paylaşarak daha fazla kullanıcının cihaz aldatma konusunda daha iyi sonuçlar elde etmesine yardımcı olabilirsiniz! Unutmayın ki her bir yapılandırma dosyası bile önemlidir :)

## Teşekkürler 🙏

Bu proje, geliştirme aşamasında aşağıdaki mükemmel projelerden yararlanmaktadır:

- [zygisk-dump-dex](https://github.com/ri-char/zygisk-dump-dex) - Rust Zygisk modülü geliştirme için prototip referansı sunar
- [zygisk-api-rs](https://github.com/rmnscnce/zygisk-api-rs) - Zygisk API için Rust bağımlılık desteği sağlar
- [MiPushZygisk](https://github.com/wushidia/MiPushZygisk) - Zygisk cihaz aldatma çözümleri için referans sağlar

Bu projelerin geliştiricilerine teşekkürler! 💖

---

**📱 Cihazlar, uygulama model kısıtlamalarıyla sınırlanmasın!** 🚀

> 💝 Bu modül size yardımcı olduysa, destek olmak için lütfen bir ⭐ verin
