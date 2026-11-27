//! Model Sekolah: sekolah tujuan (fiktif), kuota, dan jalur pendaftaran.

#[derive(Clone, Debug)]
pub struct Sekolah {
    pub kode: &'static str,
    pub nama: &'static str,
    pub kecamatan: &'static str,
    pub kuota: u32,
}

pub const DAFTAR_SEKOLAH: &[Sekolah] = &[
    Sekolah { kode: "SMPN1", nama: "SMP Negeri 1 Nusantara", kecamatan: "Kota Lama", kuota: 288 },
    Sekolah { kode: "SMPN2", nama: "SMP Negeri 2 Nusantara", kecamatan: "Pelabuhan", kuota: 256 },
    Sekolah { kode: "SMPN3", nama: "SMP Negeri 3 Nusantara", kecamatan: "Bukit Hijau", kuota: 224 },
    Sekolah { kode: "SMPN4", nama: "SMP Negeri 4 Nusantara", kecamatan: "Tepian Sungai", kuota: 192 },
];

pub fn cari(kode: &str) -> Option<&'static Sekolah> {
    DAFTAR_SEKOLAH.iter().find(|s| s.kode == kode)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Jalur {
    Domisili,
    Afirmasi,
    Prestasi,
    Mutasi,
}

impl Jalur {
    pub const SEMUA: [Jalur; 4] = [Jalur::Domisili, Jalur::Afirmasi, Jalur::Prestasi, Jalur::Mutasi];

    pub fn kode(self) -> &'static str {
        match self {
            Jalur::Domisili => "domisili",
            Jalur::Afirmasi => "afirmasi",
            Jalur::Prestasi => "prestasi",
            Jalur::Mutasi => "mutasi",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Jalur::Domisili => "Domisili",
            Jalur::Afirmasi => "Afirmasi",
            Jalur::Prestasi => "Prestasi",
            Jalur::Mutasi => "Mutasi orang tua",
        }
    }

    pub fn dari_kode(kode: &str) -> Option<Jalur> {
        Jalur::SEMUA.into_iter().find(|j| j.kode() == kode)
    }
}
