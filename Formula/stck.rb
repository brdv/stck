class Stck < Formula
  desc "CLI for stacked GitHub pull request workflows"
  homepage "https://github.com/brdv/stck"
  url "https://github.com/brdv/stck/releases/download/v0.1.5/stck-v0.1.5-aarch64-apple-darwin.tar.gz"
  version "0.1.5"
  sha256 "15acda6bdbf8e2cba3ab7c2d16cbc78c8723de9989686fcfb2b78fa1973f1375"

  depends_on arch: :arm64

  def install
    bin.install "stck"
    bin.install_symlink "stck" => "git-stck"
  end

  test do
    assert_match "stck", shell_output("#{bin}/stck --help")
  end
end
