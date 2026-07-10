# Homebrew formula TEMPLATE for the sylin-org/homebrew-tap repository (Formula/ghostlight.rb).
# Fill the four sha256 values from the release's .sha256 assets, then push to the tap.
# Users: brew install sylin-org/tap/ghostlight
class Ghostlight < Formula
  desc "Governed browser automation over your own authenticated Chromium session (MCP)"
  homepage "https://sylin-org.github.io/ghostlight/"
  version "0.5.4"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/sylin-org/ghostlight/releases/download/v#{version}/ghostlight-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "TODO-SHA256-FROM-RELEASE-ASSET"
    else
      url "https://github.com/sylin-org/ghostlight/releases/download/v#{version}/ghostlight-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "TODO-SHA256-FROM-RELEASE-ASSET"
    end
  end

  on_linux do
    url "https://github.com/sylin-org/ghostlight/releases/download/v#{version}/ghostlight-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "TODO-SHA256-FROM-RELEASE-ASSET"
  end

  def install
    # ADR-0046 as amended by ADR-0051: two executables ship in the archive
    # (ghostlight + the single role-selected ghostlight-relay pass-through).
    bin.install "ghostlight", "ghostlight-relay"
  end

  def caveats
    <<~EOS
      Connect the browser side (idempotent):
        ghostlight install
      then add the "Ghostlight in Browser" extension.
      Walkthrough: https://sylin-org.github.io/ghostlight/install.html
    EOS
  end

  test do
    system "#{bin}/ghostlight", "--version"
  end
end
