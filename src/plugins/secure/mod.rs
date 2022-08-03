
/// Component for dealing with secure data, 
/// 
/// This component is mainly best effort, and is not trying to reinvent any security protocols.
/// Underneath the hood it will use the securestore-rs library to enable this feature.
/// 
/// The focus is for dealing with runtime secrets, so any type of critical secrets need to be stored at rest
/// on actual vault services (such as .key files) such as Azure Key Vault. 
/// 
/// This component will focus on transient secrets, like access tokens, shared access tokens, etc.
/// 
/// # About securestore-rs 
/// 
/// securestore-rs' main data artifact is the secrets.json file which is an opaque archive of the secret store, designed to be checked-in
/// and saved in plain text, and transferred between machines. 
/// 
/// The secret artifact is a .key file that must be protected and stored in a vault service. 
///
pub struct Secure;

pub trait TokenProvider {
    fn get_token(&self) -> Secure; 
}
