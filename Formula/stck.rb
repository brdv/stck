class Stck < Formula
  desc "CLI for stacked GitHub pull request workflows"
  homepage "https://github.com/brdv/stck"
  version "0.1.0"
  depends_on arch: :arm64

  url "https://github.com/brdv/stck/releases/download/v#{version}/stck-v#{version}-aarch64-apple-darwin.tar.gz"
  # Replace this SHA256 when publishing a new release.
  sha256 "8461c4c824a619aa2f9209dad1f602fed2b4e0f05538e27a22fece249601adc7"

  def install
    bin.install "stck"
    bin.install_symlink "stck" => "git-stck"
  end

  test do
    assert_match "stck", shell_output("#{bin}/stck --help")
  end
end
