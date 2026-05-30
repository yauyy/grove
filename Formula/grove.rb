class Grove < Formula
  desc "Multi-project git worktree workspace manager"
  homepage "https://github.com/yauyy/grove"
  version "0.8.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-aarch64-apple-darwin.tar.gz"
      sha256 "dabb7b9a7378db1eaf95ede559780bcaa189ea76fa4630918a3be7bfa6d419e7"
    else
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-apple-darwin.tar.gz"
      sha256 "c6517c142481c8df12b767ff48c71f7621cb641b9527e343aab9944b107144de"
    end
  end

  on_linux do
    url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "8db5c1f809c0dba1124c7c7c79eababf9e6333fcea8b7c3dffaab5dd222a0325"
  end

  def install
    bin.install "grove"
  end

  test do
    assert_match "Multi-project git worktree workspace manager", shell_output("#{bin}/grove --help")
  end
end
