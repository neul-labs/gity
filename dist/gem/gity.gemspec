Gem::Specification.new do |spec|
  spec.name          = "gity"
  spec.version       = "0.1.0"
  spec.authors       = ["Neul Labs"]
  spec.email         = ["info@neul-labs.com"]

  spec.summary       = "Make large Git repositories feel instant"
  spec.description   = "A lightweight daemon that accelerates Git operations on large repositories by implementing fsmonitor and caching"
  spec.homepage      = "https://github.com/neul-labs/gity"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 2.6.0"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/neul-labs/gity"
  spec.metadata["changelog_uri"] = "https://github.com/neul-labs/gity/blob/main/CHANGELOG.md"

  spec.files = Dir["lib/**/*", "exe/*", "README.md", "LICENSE"]
  spec.bindir = "exe"
  spec.executables = ["gity"]
  spec.require_paths = ["lib"]

  spec.extensions = ["ext/extconf.rb"]

  spec.add_dependency "os", "~> 1.1"
end
