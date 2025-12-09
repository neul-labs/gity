Name:           gity
Version:        0.1.0
Release:        1%{?dist}
Summary:        Make large Git repositories feel instant

License:        MIT
URL:            https://github.com/neul-labs/gity
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
Requires:       git >= 2.37

%description
Gity is a lightweight, cross-platform daemon that accelerates Git
operations on large repositories. It watches your files, implements
Git's fsmonitor protocol, runs background maintenance, and caches
status results for instant responses.

%prep
%autosetup

%build
cargo build --release

%install
install -Dm755 target/release/gity %{buildroot}%{_bindir}/gity

%files
%license LICENSE
%doc README.md CHANGELOG.md
%{_bindir}/gity

%changelog
* Mon Dec 09 2024 Neul Labs <info@neul-labs.com> - 0.1.0-1
- Initial release
