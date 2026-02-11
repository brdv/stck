class Stck < Formula
  desc "CLI for stacked GitHub pull request workflows"
  homepage "https://github.com/brdv/stck"
  version "0.1.0"
  depends_on arch: :arm64

  url "https://github.com/brdv/stck/releases/download/v#{version}/stck-v#{version}-aarch64-apple-darwin.tar.gz"
  # Replace this SHA256 when publishing a new release.
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  def install
    bin.install "stck"
    bin.install_symlink "stck" => "git-stck"
  end

  test do
    assert_match "stck", shell_output("#{bin}/stck --help")
  end
end
