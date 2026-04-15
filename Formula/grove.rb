class Grove < Formula
  desc "Multi-project git worktree workspace manager"
  homepage "https://github.com/yauyy/grove"
  version "0.4.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-aarch64-apple-darwin.tar.gz"
      sha256 "10208a353c478235bf4a6f7858a48436d8b34e1a3d28c5ca16ccc77471785058"
    else
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-apple-darwin.tar.gz"
      sha256 "5658e20e56dae62a64f6356a19cb1fb30f52651d0725fc63f5548048b8edeca2"
    end
  end

  on_linux do
    url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "b6f012ecac38d2682672603edf35aa112c7dd844931149df079a6286c3ac4365"
  end

  def install
    bin.install "grove"
  end

  test do
    assert_match "Multi-project git worktree workspace manager", shell_output("#{bin}/grove --help")
  end
end
