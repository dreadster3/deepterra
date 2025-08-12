use super::Result;
use crate::terraform;

pub struct GithubParser {}

impl GithubParser {
    pub async fn parse(scope: &str) -> Result<terraform::TerraformManifest> {
        todo!()
    }
}
