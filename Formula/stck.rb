class Stck < Formula
  desc "CLI for stacked GitHub pull request workflows"
  homepage "https://github.com/brdv/stck"
  version "0.1.0"
  depends_on arch: :arm64

  url "https://github.com/brdv/stck/releases/download/v#{version}/stck-v#{version}-aarch64-apple-darwin.tar.gz"
  # Replace this SHA256 when publishing a new release.
  sha256 "3e42623a90d342b16a322df00ed8d4ab1249b8e467d8f33e40b6fa1fff2c16eb"

  def install
    bin.install "stck"
    bin.install_symlink "stck" => "git-stck"
  end

  test do
    assert_match "stck", shell_output("#{bin}/stck --help")
  end
end
