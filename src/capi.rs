use std::time::Duration;
use std::error::Error;
use crate::models::*;
use reqwest::StatusCode;
use std::fmt::Display;
use std::collections::HashMap;
use itertools::Itertools;


/// CapiError represents parsed errors from the Content API
/// It is compatible with Error, contains code and message, can be printed and has a method "should_retry" indicating if the error is retryable or not
///
/// To see if an error is a CapiError, you can:
/// 
/// ```rust
/// match err.downcast_ref::<CapiError>() {
///   Some(capi_err)=>println!("{} can retry? {}", capi_err, capi_err.should_retry()),
///   None=>println!("Not a CAPI error")
/// }
/// ```
#[derive(Debug)]
pub struct CapiError {
    code:u16,
    msg:String
}

impl Display for CapiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("CAPI error {}: {}", self.code, self.msg))
    }
}

impl Error for CapiError {

}

impl CapiError {
    pub fn new(code:StatusCode, msg:&str) -> CapiError {
        CapiError { code: code.as_u16(), msg: msg.to_owned() }
    }

    pub fn should_retry(&self) -> bool {
        self.code==503 || self.code==504
    }
}

async fn internal_make_request(client: &reqwest::Client, url:&str) -> Result<CapiResponseEnvelope, Box<dyn Error>> {
    let response = client.get(url).send().await?;
    let status = response.status();
    let body = response.bytes().await?;

    if status==200 {
        let ds = &mut serde_json::Deserializer::from_slice(&body);
        match serde_path_to_error::deserialize(ds) {
            Ok(content)=>return Ok(content),
            Err(e)=>{
                println!("ERROR could not unmarshal content: {}", e);
                let body_string:Vec<u8> = body.into_iter().collect();
                let content_string = String::from_utf8(body_string).unwrap_or(String::from("(not utf)"));
                println!("Body was: {}", content_string);
                return Err(Box::new(e))
            }
        }
    } else {
        let content = std::str::from_utf8(&body).unwrap_or("invalid UTF data");
        return Err(Box::new(CapiError::new(status, content)));
    }
}

/// Public method to request content from the Content Application Programmer's Interface.
/// # Arguments:
/// 
/// * `client` - Immutable reference to an HTTP client (provided by Reqwest) for making the http requests with
/// * `capi_key` - String of the API key to use
/// * `query_tag` - Tags query to use. This takes the form of a comma-separated list of tag IDs (for AND) or a pipe-separated list of tag IDs (for OR). Any tag ID can be negated by appending a - sign
/// * `page_counter` - Number of the page to retrieve. Pages start at 1.
/// * `page_size` - Number of items to retrieve on a page
/// * `retry_delay` - a Duration representing the amount of time to wait between unsuccessful requests. Note that there is no retry for 4xx requests.
pub async fn make_capi_request(client: &reqwest::Client, capi_key:String, query_tag:String, page_counter:u64, page_size:u32, retry_delay:Option<Duration>, max_attempts:Option<i32>, base_url:Option<String>) -> Result<CapiResponseEnvelope, Box<dyn Error>> {
    let args = HashMap::from([
        ("api-key", capi_key),
        ("show-tags", String::from("all")),
        ("tag", query_tag),
        ("show-blocks", String::from("all")),
        ("page", format!("{}", page_counter)),
        ("page-size", format!("{}", page_size))
    ]);

    let argstring:String = args.iter()
        .map(|(k,v)| format!("{}={}", k, url_escape::encode_fragment(v)))
        .intersperse(String::from("&"))
        .collect();
    
    let url = format!("{}/search?{}", base_url.unwrap_or(String::from("https://content.guardianapis.com")), argstring);

    let mut attempts = 0;
    loop {
        attempts += 1;
        match internal_make_request(client, &url).await {
            Ok(content)=>return Ok(content),
            Err(err)=>
                match err.downcast_ref::<CapiError>() {
                    Some(capi_err)=>
                        if capi_err.should_retry() {
                            if attempts >= max_attempts.unwrap_or(10) {
                                return Err(err);
                            } else {
                                std::thread::sleep(retry_delay.unwrap_or(Duration::from_secs(2)));
                                continue;
                            }
                        } else {
                            return Err(err);
                        },
                    None=>return Err(err)
                }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use reqwest::Client;

    const SUCCESS_RESPONSE:&str = r#"{
        "response": {
            "status": "ok",
            "userTier": "internal",
            "total": 28287,
            "startIndex": 1,
            "pageSize": 1,
            "currentPage": 1,
            "pages": 28287,
            "orderBy": "newest",
            "results": [
                {
                    "id": "artanddesign/2023/oct/13/the-week-in-art",
                    "type": "article",
                    "sectionId": "artanddesign",
                    "sectionName": "Art and design",
                    "webPublicationDate": "2023-10-13T12:22:26Z",
                    "webTitle": "Japan’s floating world, Britain’s lakes of paint and California’s sculpted light – the week in art",
                    "webUrl": "https://www.theguardian.com/artanddesign/2023/oct/13/the-week-in-art",
                    "apiUrl": "https://content.guardianapis.com/artanddesign/2023/oct/13/the-week-in-art",
                    "tags": [
                        {
                            "id": "artanddesign/series/art-weekly",
                            "type": "series",
                            "sectionId": "artanddesign",
                            "sectionName": "Art and design",
                            "webTitle": "Art Weekly",
                            "webUrl": "https://www.theguardian.com/artanddesign/series/art-weekly",
                            "apiUrl": "https://content.guardianapis.com/artanddesign/series/art-weekly",
                            "references": [],
                            "description": "Your weekly art world low-down, sketching out news, ideas and things to see this week. <a href=\"http://www.guardian.co.uk/artanddesign/signup/2011/jul/08/art-weekly-newsletter-sign-up\">Sign up to the newsletter here</a>",
                            "internalName": "Art Weekly"
                        },
                        {
                            "id": "artanddesign/artanddesign",
                            "type": "keyword",
                            "sectionId": "artanddesign",
                            "sectionName": "Art and design",
                            "webTitle": "Art and design",
                            "webUrl": "https://www.theguardian.com/artanddesign/artanddesign",
                            "apiUrl": "https://content.guardianapis.com/artanddesign/artanddesign",
                            "references": [],
                            "internalName": "Art and design"
                        },
                        {
                            "id": "culture/culture",
                            "type": "keyword",
                            "sectionId": "culture",
                            "sectionName": "Culture",
                            "webTitle": "Culture",
                            "webUrl": "https://www.theguardian.com/culture/culture",
                            "apiUrl": "https://content.guardianapis.com/culture/culture",
                            "references": [],
                            "internalName": "Culture"
                        },
                        {
                            "id": "artanddesign/painting",
                            "type": "keyword",
                            "sectionId": "artanddesign",
                            "sectionName": "Art and design",
                            "webTitle": "Painting",
                            "webUrl": "https://www.theguardian.com/artanddesign/painting",
                            "apiUrl": "https://content.guardianapis.com/artanddesign/painting",
                            "references": [],
                            "internalName": "Painting (Art and design)"
                        },
                        {
                            "id": "artanddesign/photography",
                            "type": "keyword",
                            "sectionId": "artanddesign",
                            "sectionName": "Art and design",
                            "webTitle": "Photography",
                            "webUrl": "https://www.theguardian.com/artanddesign/photography",
                            "apiUrl": "https://content.guardianapis.com/artanddesign/photography",
                            "references": [],
                            "internalName": "Photography (Art and design)"
                        },
                        {
                            "id": "artanddesign/art",
                            "type": "keyword",
                            "sectionId": "artanddesign",
                            "sectionName": "Art and design",
                            "webTitle": "Art",
                            "webUrl": "https://www.theguardian.com/artanddesign/art",
                            "apiUrl": "https://content.guardianapis.com/artanddesign/art",
                            "references": [],
                            "internalName": "Art (visual arts only)"
                        },
                        {
                            "id": "artanddesign/exhibition",
                            "type": "keyword",
                            "sectionId": "artanddesign",
                            "sectionName": "Art and design",
                            "webTitle": "Exhibitions",
                            "webUrl": "https://www.theguardian.com/artanddesign/exhibition",
                            "apiUrl": "https://content.guardianapis.com/artanddesign/exhibition",
                            "references": [],
                            "internalName": "Exhibitions"
                        },
                        {
                            "id": "world/japan",
                            "type": "keyword",
                            "sectionId": "world",
                            "sectionName": "World news",
                            "webTitle": "Japan",
                            "webUrl": "https://www.theguardian.com/world/japan",
                            "apiUrl": "https://content.guardianapis.com/world/japan",
                            "references": [],
                            "internalName": "Japan (News)"
                        },
                        {
                            "id": "technology/artificialintelligenceai",
                            "type": "keyword",
                            "sectionId": "technology",
                            "sectionName": "Technology",
                            "webTitle": "Artificial intelligence (AI)",
                            "webUrl": "https://www.theguardian.com/technology/artificialintelligenceai",
                            "apiUrl": "https://content.guardianapis.com/technology/artificialintelligenceai",
                            "references": [],
                            "internalName": "Artificial intelligence (Technology)"
                        },
                        {
                            "id": "artanddesign/van-dyck",
                            "type": "keyword",
                            "sectionId": "artanddesign",
                            "sectionName": "Art and design",
                            "webTitle": "Van Dyck",
                            "webUrl": "https://www.theguardian.com/artanddesign/van-dyck",
                            "apiUrl": "https://content.guardianapis.com/artanddesign/van-dyck",
                            "references": [],
                            "internalName": "Van Dyck"
                        },
                        {
                            "id": "type/article",
                            "type": "type",
                            "webTitle": "Article",
                            "webUrl": "https://www.theguardian.com/articles",
                            "apiUrl": "https://content.guardianapis.com/type/article",
                            "references": [],
                            "internalName": "Article (Content type)"
                        },
                        {
                            "id": "tone/features",
                            "type": "tone",
                            "webTitle": "Features",
                            "webUrl": "https://www.theguardian.com/tone/features",
                            "apiUrl": "https://content.guardianapis.com/tone/features",
                            "references": [],
                            "internalName": "Feature (Tone)"
                        },
                        {
                            "id": "profile/jonathanjones",
                            "type": "contributor",
                            "webTitle": "Jonathan Jones",
                            "webUrl": "https://www.theguardian.com/profile/jonathanjones",
                            "apiUrl": "https://content.guardianapis.com/profile/jonathanjones",
                            "references": [],
                            "bio": "<p>Jonathan Jones writes on art for the Guardian and was on the jury for the 2009 Turner prize</p>",
                            "bylineImageUrl": "https://static.guim.co.uk/sys-images/Guardian/Pix/pictures/2014/4/17/1397749334203/JonathanJones.jpg",
                            "bylineLargeImageUrl": "https://uploads.guim.co.uk/2017/10/06/Jonathan-Jones,-L.png",
                            "firstName": "jones",
                            "lastName": "jonathan",
                            "rcsId": "GNL234013",
                            "r2ContributorId": "15906",
                            "internalName": "Jonathan Jones"
                        },
                        {
                            "id": "tracking/commissioningdesk/uk-culture",
                            "type": "tracking",
                            "webTitle": "UK Culture",
                            "webUrl": "https://www.theguardian.com/tracking/commissioningdesk/uk-culture",
                            "apiUrl": "https://content.guardianapis.com/tracking/commissioningdesk/uk-culture",
                            "references": [],
                            "internalName": "UK Culture (commissioning)"
                        }
                    ],
                    "blocks": {
                        "main": {
                            "id": "6527e7ef8f0830b0d91d87b3",
                            "bodyHtml": "<figure class=\"element element-image\" data-media-id=\"fdf09841c2265235e897069abe433439e6521fac\"> <img src=\"https://media.guim.co.uk/fdf09841c2265235e897069abe433439e6521fac/0_0_2693_1882/1000.jpg\" alt=\"The Treasure Ship by Utagawa Hiroshige (1797-1858), depicting the seven gods of fortune (c 1840).\" width=\"1000\" height=\"699\" class=\"gu-image\" /> <figcaption> <span class=\"element-image__caption\">The Treasure Ship by Utagawa Hiroshige (1797-1858), depicting the seven gods of fortune (c 1840). </span> <span class=\"element-image__credit\">Photograph: Akarma/Victoria and Albert Museum, London</span> </figcaption> </figure>",
                            "bodyTextSummary": "",
                            "attributes": {},
                            "published": true,
                            "createdDate": "2023-10-13T12:22:26Z",
                            "lastModifiedDate": "2023-10-12T16:55:15Z",
                            "contributors": [],
                            "createdBy": {
                                "email": "lindesay.irvine@guardian.co.uk",
                                "firstName": "Lindesay",
                                "lastName": "Irvine"
                            },
                            "lastModifiedBy": {
                                "email": "alex.barlow.casual@guardian.co.uk",
                                "firstName": "Alex",
                                "lastName": "Barlow"
                            },
                            "elements": [
                                {
                                    "type": "image",
                                    "assets": [
                                        {
                                            "type": "image",
                                            "mimeType": "image/jpeg",
                                            "file": "https://media.guim.co.uk/fdf09841c2265235e897069abe433439e6521fac/0_0_2693_1882/2693.jpg",
                                            "typeData": {
                                                "width": 2693,
                                                "height": 1882
                                            }
                                        },
                                        {
                                            "type": "image",
                                            "mimeType": "image/jpeg",
                                            "file": "https://media.guim.co.uk/fdf09841c2265235e897069abe433439e6521fac/0_0_2693_1882/master/2693.jpg",
                                            "typeData": {
                                                "width": 2693,
                                                "height": 1882,
                                                "isMaster": true
                                            }
                                        },
                                        {
                                            "type": "image",
                                            "mimeType": "image/jpeg",
                                            "file": "https://media.guim.co.uk/fdf09841c2265235e897069abe433439e6521fac/0_0_2693_1882/2000.jpg",
                                            "typeData": {
                                                "width": 2000,
                                                "height": 1398
                                            }
                                        },
                                        {
                                            "type": "image",
                                            "mimeType": "image/jpeg",
                                            "file": "https://media.guim.co.uk/fdf09841c2265235e897069abe433439e6521fac/0_0_2693_1882/1000.jpg",
                                            "typeData": {
                                                "width": 1000,
                                                "height": 699
                                            }
                                        },
                                        {
                                            "type": "image",
                                            "mimeType": "image/jpeg",
                                            "file": "https://media.guim.co.uk/fdf09841c2265235e897069abe433439e6521fac/0_0_2693_1882/500.jpg",
                                            "typeData": {
                                                "width": 500,
                                                "height": 349
                                            }
                                        },
                                        {
                                            "type": "image",
                                            "mimeType": "image/jpeg",
                                            "file": "https://media.guim.co.uk/fdf09841c2265235e897069abe433439e6521fac/0_0_2693_1882/140.jpg",
                                            "typeData": {
                                                "width": 140,
                                                "height": 98
                                            }
                                        }
                                    ],
                                    "imageTypeData": {
                                        "caption": "The Treasure Ship by Utagawa Hiroshige (1797-1858), depicting the seven gods of fortune (c 1840). ",
                                        "displayCredit": true,
                                        "credit": "Photograph: Akarma/Victoria and Albert Museum, London",
                                        "source": "Victoria and Albert Museum, London",
                                        "photographer": "Akarma",
                                        "alt": "The Treasure Ship by Utagawa Hiroshige (1797-1858), depicting the seven gods of fortune (c 1840).",
                                        "mediaId": "fdf09841c2265235e897069abe433439e6521fac",
                                        "mediaApiUri": "https://api.media.gutools.co.uk/images/fdf09841c2265235e897069abe433439e6521fac",
                                        "imageType": "Photograph"
                                    }
                                }
                            ]
                        },
                        "body": [
                            {
                                "id": "6284cc918f08d6747292664f",
                                "bodyHtml": "<h2>Exhibition of the week</h2> <p><strong>Japan: Myths to Manga</strong><strong><br></strong>Something genuinely innovative – a proper art historical show for the kids, from the floating world to modern manga.<br>• <a href=\"https://www.vam.ac.uk/exhibitions/japan-myths-to-manga\">Young V&amp;A, London, from 14 October</a></p> <h2>Also showing</h2> <p><strong>Alberta Whittle<br></strong>Whittle releases more images and ideas from her apparently limitless imagination.<br>• <a href=\"https://www.themoderninstitute.com/exhibitions/the-modern-institute-14-20-osborne-street/8585/\">Modern Institute, Glasgow, until 11 November</a></p> <p><strong>La Serenissima</strong><strong><br></strong>Lose yourself in drawings of 18th-century Venice by Canaletto and his contemporaries.<br>• <a href=\"https://courtauld.ac.uk/whats-on/la-serenissima-drawing-in-18th-century-venice/\">Courtauld Gallery, London, 14 October to 11 February</a></p> <p><strong>Ian Davenport</strong><strong><br></strong>New works by that rarest of beings, a serious British abstract painter.<br>• <a href=\"https://www.waddingtoncustot.com/exhibitions/203/\">Waddington Custot, London, until 11 November</a></p> <p><strong>Robert Irwin and Mary Corse</strong><strong><br></strong>Blow your mind with vintage California light art.<br>• <a href=\"https://www.pacegallery.com/exhibitions/robert-irwin-and-mary-corse-parallax/\">Pace, London, until 11 November</a></p> <h2>Image of the week</h2>  <figure class=\"element element-image element--showcase\" data-media-id=\"d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b\"> <img src=\"https://media.guim.co.uk/d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b/0_0_14998_5648/1000.jpg\" alt=\"Gassed, 1918, by John Singer Sargent\" width=\"1000\" height=\"377\" class=\"gu-image\" /> <figcaption> <span class=\"element-image__caption\">Gassed, 1918, by John Singer Sargent.</span> <span class=\"element-image__credit\">Photograph: IWM</span> </figcaption> </figure>  <p>Enormous in scale – it is over six metres wide – Gassed, by John Singer Sargent, depicts lines of soldiers, blinded by mustard gas, picking their way through a crowded battlefield, each with a hand on the shoulder of the man in front. The era-defining artwork has been newly restored and will be <a href=\"https://www.theguardian.com/artanddesign/2023/oct/12/it-glows-restorer-removes-queasy-look-from-first-world-war-painting-gassed\">going on display at the IWM London</a> on 10 November.</p> <h2>What we learned</h2> <p><a href=\"https://www.theguardian.com/artanddesign/gallery/2023/oct/12/real-or-imagined-ngv-examines-both-sides-of-photography-in-pictures\">Photography has been deepfaking us since the 1800s</a></p> <p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/12/mr-eazi-evil-genius-debut-album-afrobeats\">Afrobeats star Mr Eazi has turned his latest album into an art show</a></p> <p><a href=\"https://www.theguardian.com/world/2023/oct/08/lost-mirror-jews-conversos-medieval-spain-prado-madrid\">Madrid’s Prado museum shows how images shaped Jewish-Christian relationships</a></p> <p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/09/astonishing-art-and-life-of-nicole-eisenman-porn\">Artist Nicole Eisenman prefers grappling with paint than isms</a></p> <p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/09/el-anatsui-turbine-hall-tate-recycled-rubbish\">El Anatsui has made gleaming miracles from rubbish at Tate Modern’s Turbine Hall</a></p> <p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/10/hiroshi-sugimoto-review-japan-great-faker-hayward\">Hiroshi Sugimoto brings the dead back to disturbing life at his new London show</a></p> <p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/10/famous-forgotten-frieze-lost-female-painters\">Frieze art fair is hoping to write women back into the story of art</a></p> <p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/11/colourful-beauty-parthenon-marbles-revealed-scientific-analysis\">Scientists have found clues to the Parthenon marbles’ original bright colours</a></p> <p><a href=\"https://www.theguardian.com/technology/gallery/2023/oct/09/ai-and-the-landscapes-of-capability-brown-in-pictures\">AI has queasily interpreted the landscapes of Capability Brown</a></p> <h2>Masterpiece of the week</h2> <p><strong>Portrait of the artist </strong><strong>Sofonisba Anguissola</strong><strong> by </strong><strong>Anthony van Dyck</strong><strong>, 1624</strong></p>  <figure class=\"element element-image\" data-media-id=\"733c7f1db3ed8d948238c0c6a87e7bb944fa10f6\"> <img src=\"https://media.guim.co.uk/733c7f1db3ed8d948238c0c6a87e7bb944fa10f6/0_0_2000_2550/784.jpg\" alt=\"Sofonisba Anguissola, 1624, by Anthony van Dyck\" width=\"784\" height=\"1000\" class=\"gu-image\" /> </figure>  <p>This portrait of a 96-year-old woman was once misidentified as one of Van Dyck’s many paintings of the British aristocracy. But his original drawing in the British Museum, dated 12 July 1624, reveals who it really depicts – and makes this a precious document of one of the first professional female artists. Sofonisba Anguissola was born in Cremona, Italy, to upper-class parents who decided all their daughters should have an art education. She proved the most gifted, and painted a Renaissance masterpiece, <a href=\"https://en.wikipedia.org/wiki/The_Game_of_Chess_%28Sofonisba_Anguissola%29#/media/File:Sofonisba_Anguissola_-_Portrait_of_the_Artist's_Sisters_Playing_Chess_-_WGA00697.jpg\">The Game of Chess</a>, when she was in her 20s. She went on to impress Michelangelo and be celebrated in the second edition of Vasari’s Lives of the Artists. Here this renowned portrait painter poses for a fan who sought her out.<br>• <a href=\"https://www.nationaltrustcollections.org.uk/object/129883\">Knole, Kent, National Trust</a></p> <h2><strong>Don’t forget</strong></h2> <p>To follow us on X (Twitter): <a href=\"https://twitter.com/Gdnartanddesign?ref_src=twsrc%5Egoogle%7Ctwcamp%5Eserp%7Ctwgr%5Eauthor\">@GdnArtandDesign</a>.</p> <h2><strong>Sign up to the Art Weekly newsletter</strong></h2> <p>If you don’t already receive our regular roundup of art and design news via email, <a href=\"https://www.theguardian.com/artanddesign/2015/oct/19/sign-up-to-the-art-weekly-email\">please sign up here</a>.</p> <h2><strong>Get in Touch</strong></h2> <p>If you have any questions or comments about any of our newsletters please email <a href=\"mailto:newsletters@theguardian.com\">newsletters@theguardian.com</a></p>",
                                "bodyTextSummary": "Exhibition of the week Japan: Myths to Manga Something genuinely innovative – a proper art historical show for the kids, from the floating world to modern manga. • Young V&A, London, from 14 October Also showing Alberta Whittle Whittle releases more images and ideas from her apparently limitless imagination. • Modern Institute, Glasgow, until 11 November La Serenissima Lose yourself in drawings of 18th-century Venice by Canaletto and his contemporaries. • Courtauld Gallery, London, 14 October to 11 February Ian Davenport New works by that rarest of beings, a serious British abstract painter. • Waddington Custot, London, until 11 November Robert Irwin and Mary Corse Blow your mind with vintage California light art. • Pace, London, until 11 November Image of the week\nEnormous in scale – it is over six metres wide – Gassed, by John Singer Sargent, depicts lines of soldiers, blinded by mustard gas, picking their way through a crowded battlefield, each with a hand on the shoulder of the man in front. The era-defining artwork has been newly restored and will be going on display at the IWM London on 10 November. What we learned Photography has been deepfaking us since the 1800s Afrobeats star Mr Eazi has turned his latest album into an art show Madrid’s Prado museum shows how images shaped Jewish-Christian relationships Artist Nicole Eisenman prefers grappling with paint than isms El Anatsui has made gleaming miracles from rubbish at Tate Modern’s Turbine Hall Hiroshi Sugimoto brings the dead back to disturbing life at his new London show Frieze art fair is hoping to write women back into the story of art Scientists have found clues to the Parthenon marbles’ original bright colours AI has queasily interpreted the landscapes of Capability Brown Masterpiece of the week Portrait of the artist Sofonisba Anguissola by Anthony van Dyck, 1624\nThis portrait of a 96-year-old woman was once misidentified as one of Van Dyck’s many paintings of the British aristocracy. But his original drawing in the British Museum, dated 12 July 1624, reveals who it really depicts – and makes this a precious document of one of the first professional female artists. Sofonisba Anguissola was born in Cremona, Italy, to upper-class parents who decided all their daughters should have an art education. She proved the most gifted, and painted a Renaissance masterpiece, The Game of Chess, when she was in her 20s. She went on to impress Michelangelo and be celebrated in the second edition of Vasari’s Lives of the Artists. Here this renowned portrait painter poses for a fan who sought her out. • Knole, Kent, National Trust Don’t forget To follow us on X (Twitter): @GdnArtandDesign. Sign up to the Art Weekly newsletter If you don’t already receive our regular roundup of art and design news via email, please sign up here. Get in Touch If you have any questions or comments about any of our newsletters please email newsletters@theguardian.com",
                                "attributes": {},
                                "published": true,
                                "createdDate": "2023-10-13T12:22:26Z",
                                "lastModifiedDate": "2023-10-13T08:42:54Z",
                                "contributors": [],
                                "createdBy": {
                                    "email": "nigel.pollitt.casual@guardian.co.uk",
                                    "firstName": "Nigel",
                                    "lastName": "Pollitt"
                                },
                                "lastModifiedBy": {
                                    "email": "john-paul.nicholas@guardian.co.uk",
                                    "firstName": "John-Paul",
                                    "lastName": "Nicholas"
                                },
                                "elements": [
                                    {
                                        "type": "text",
                                        "assets": [],
                                        "textTypeData": {
                                            "html": "<h2>Exhibition of the week</h2> \n<p><strong>Japan: Myths to Manga</strong><strong><br></strong>Something genuinely innovative – a proper art historical show for the kids, from the floating world to modern manga.<br>• <a href=\"https://www.vam.ac.uk/exhibitions/japan-myths-to-manga\">Young V&amp;A, London, from 14 October</a></p> \n<h2>Also showing</h2> \n<p><strong>Alberta Whittle<br></strong>Whittle releases more images and ideas from her apparently limitless imagination.<br>• <a href=\"https://www.themoderninstitute.com/exhibitions/the-modern-institute-14-20-osborne-street/8585/\">Modern Institute, Glasgow, until 11 November</a></p> \n<p><strong>La Serenissima</strong><strong><br></strong>Lose yourself in drawings of 18th-century Venice by Canaletto and his contemporaries.<br>• <a href=\"https://courtauld.ac.uk/whats-on/la-serenissima-drawing-in-18th-century-venice/\">Courtauld Gallery, London, 14 October to 11 February</a></p> \n<p><strong>Ian Davenport</strong><strong><br></strong>New works by that rarest of beings, a serious British abstract painter.<br>• <a href=\"https://www.waddingtoncustot.com/exhibitions/203/\">Waddington Custot, London, until 11 November</a></p> \n<p><strong>Robert Irwin and Mary Corse</strong><strong><br></strong>Blow your mind with vintage California light art.<br>• <a href=\"https://www.pacegallery.com/exhibitions/robert-irwin-and-mary-corse-parallax/\">Pace, London, until 11 November</a></p> \n<h2>Image of the week</h2>"
                                        }
                                    },
                                    {
                                        "type": "image",
                                        "assets": [
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b/0_0_14998_5648/14998.jpg",
                                                "typeData": {
                                                    "width": 14998,
                                                    "height": 5648
                                                }
                                            },
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b/0_0_14998_5648/master/14998.jpg",
                                                "typeData": {
                                                    "width": 14998,
                                                    "height": 5648,
                                                    "isMaster": true
                                                }
                                            },
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b/0_0_14998_5648/2000.jpg",
                                                "typeData": {
                                                    "width": 2000,
                                                    "height": 753
                                                }
                                            },
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b/0_0_14998_5648/1000.jpg",
                                                "typeData": {
                                                    "width": 1000,
                                                    "height": 377
                                                }
                                            },
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b/0_0_14998_5648/500.jpg",
                                                "typeData": {
                                                    "width": 500,
                                                    "height": 188
                                                }
                                            },
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b/0_0_14998_5648/140.jpg",
                                                "typeData": {
                                                    "width": 140,
                                                    "height": 53
                                                }
                                            }
                                        ],
                                        "imageTypeData": {
                                            "caption": "Gassed, 1918, by John Singer Sargent.",
                                            "displayCredit": true,
                                            "credit": "Photograph: IWM",
                                            "source": "IWM",
                                            "alt": "Gassed, 1918, by John Singer Sargent",
                                            "mediaId": "d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b",
                                            "mediaApiUri": "https://api.media.gutools.co.uk/images/d61aaf102b14dbc1fc2f77fe4cec4246613b4d7b",
                                            "imageType": "Photograph",
                                            "role": "showcase"
                                        }
                                    },
                                    {
                                        "type": "text",
                                        "assets": [],
                                        "textTypeData": {
                                            "html": "<p>Enormous in scale – it is over six metres wide – Gassed, by John Singer Sargent, depicts lines of soldiers, blinded by mustard gas, picking their way through a crowded battlefield, each with a hand on the shoulder of the man in front. The era-defining artwork has been newly restored and will be <a href=\"https://www.theguardian.com/artanddesign/2023/oct/12/it-glows-restorer-removes-queasy-look-from-first-world-war-painting-gassed\">going on display at the IWM London</a> on 10 November.</p> \n<h2>What we learned</h2> \n<p><a href=\"https://www.theguardian.com/artanddesign/gallery/2023/oct/12/real-or-imagined-ngv-examines-both-sides-of-photography-in-pictures\">Photography has been deepfaking us since the 1800s</a></p> \n<p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/12/mr-eazi-evil-genius-debut-album-afrobeats\">Afrobeats star Mr Eazi has turned his latest album into an art show</a></p> \n<p><a href=\"https://www.theguardian.com/world/2023/oct/08/lost-mirror-jews-conversos-medieval-spain-prado-madrid\">Madrid’s Prado museum shows how images shaped Jewish-Christian relationships</a></p> \n<p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/09/astonishing-art-and-life-of-nicole-eisenman-porn\">Artist Nicole Eisenman prefers grappling with paint than isms</a></p> \n<p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/09/el-anatsui-turbine-hall-tate-recycled-rubbish\">El Anatsui has made gleaming miracles from rubbish at Tate Modern’s Turbine Hall</a></p> \n<p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/10/hiroshi-sugimoto-review-japan-great-faker-hayward\">Hiroshi Sugimoto brings the dead back to disturbing life at his new London show</a></p> \n<p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/10/famous-forgotten-frieze-lost-female-painters\">Frieze art fair is hoping to write women back into the story of art</a></p> \n<p><a href=\"https://www.theguardian.com/artanddesign/2023/oct/11/colourful-beauty-parthenon-marbles-revealed-scientific-analysis\">Scientists have found clues to the Parthenon marbles’ original bright colours</a></p> \n<p><a href=\"https://www.theguardian.com/technology/gallery/2023/oct/09/ai-and-the-landscapes-of-capability-brown-in-pictures\">AI has queasily interpreted the landscapes of Capability Brown</a></p> \n<h2>Masterpiece of the week</h2> \n<p><strong>Portrait of the artist </strong><strong>Sofonisba Anguissola</strong><strong> by </strong><strong>Anthony van Dyck</strong><strong>, 1624</strong></p>"
                                        }
                                    },
                                    {
                                        "type": "image",
                                        "assets": [
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/733c7f1db3ed8d948238c0c6a87e7bb944fa10f6/0_0_2000_2550/2000.jpg",
                                                "typeData": {
                                                    "width": 2000,
                                                    "height": 2550
                                                }
                                            },
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/733c7f1db3ed8d948238c0c6a87e7bb944fa10f6/0_0_2000_2550/master/2000.jpg",
                                                "typeData": {
                                                    "width": 2000,
                                                    "height": 2550,
                                                    "isMaster": true
                                                }
                                            },
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/733c7f1db3ed8d948238c0c6a87e7bb944fa10f6/0_0_2000_2550/1569.jpg",
                                                "typeData": {
                                                    "width": 1569,
                                                    "height": 2000
                                                }
                                            },
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/733c7f1db3ed8d948238c0c6a87e7bb944fa10f6/0_0_2000_2550/784.jpg",
                                                "typeData": {
                                                    "width": 784,
                                                    "height": 1000
                                                }
                                            },
                                            {
                                                "type": "image",
                                                "mimeType": "image/jpeg",
                                                "file": "https://media.guim.co.uk/733c7f1db3ed8d948238c0c6a87e7bb944fa10f6/0_0_2000_2550/392.jpg",
                                                "typeData": {
                                                    "width": 392,
                                                    "height": 500
                                                }
                                            }
                                        ],
                                        "imageTypeData": {
                                            "displayCredit": true,
                                            "credit": "Photograph: Artgen/Alamy",
                                            "source": "Alamy",
                                            "photographer": "Artgen",
                                            "alt": "Sofonisba Anguissola, 1624, by Anthony van Dyck",
                                            "mediaId": "733c7f1db3ed8d948238c0c6a87e7bb944fa10f6",
                                            "mediaApiUri": "https://api.media.gutools.co.uk/images/733c7f1db3ed8d948238c0c6a87e7bb944fa10f6",
                                            "suppliersReference": "2RFYFXY",
                                            "imageType": "Photograph"
                                        }
                                    },
                                    {
                                        "type": "text",
                                        "assets": [],
                                        "textTypeData": {
                                            "html": "<p>This portrait of a 96-year-old woman was once misidentified as one of Van Dyck’s many paintings of the British aristocracy. But his original drawing in the British Museum, dated 12 July 1624, reveals who it really depicts – and makes this a precious document of one of the first professional female artists. Sofonisba Anguissola was born in Cremona, Italy, to upper-class parents who decided all their daughters should have an art education. She proved the most gifted, and painted a Renaissance masterpiece, <a href=\"https://en.wikipedia.org/wiki/The_Game_of_Chess_%28Sofonisba_Anguissola%29#/media/File:Sofonisba_Anguissola_-_Portrait_of_the_Artist's_Sisters_Playing_Chess_-_WGA00697.jpg\">The Game of Chess</a>, when she was in her 20s. She went on to impress Michelangelo and be celebrated in the second edition of Vasari’s Lives of the Artists. Here this renowned portrait painter poses for a fan who sought her out.<br>• <a href=\"https://www.nationaltrustcollections.org.uk/object/129883\">Knole, Kent, National Trust</a></p> \n<h2><strong>Don’t forget</strong></h2> \n<p>To follow us on X (Twitter): <a href=\"https://twitter.com/Gdnartanddesign?ref_src=twsrc%5Egoogle%7Ctwcamp%5Eserp%7Ctwgr%5Eauthor\">@GdnArtandDesign</a>.</p> \n<h2><strong>Sign up to the Art Weekly newsletter</strong></h2> \n<p>If you don’t already receive our regular roundup of art and design news via email, <a href=\"https://www.theguardian.com/artanddesign/2015/oct/19/sign-up-to-the-art-weekly-email\">please sign up here</a>.</p> \n<h2><strong>Get in Touch</strong></h2> \n<p>If you have any questions or comments about any of our newsletters please email <a href=\"mailto:newsletters@theguardian.com\">newsletters@theguardian.com</a></p>"
                                        }
                                    }
                                ]
                            }
                        ],
                        "totalBodyBlocks": 1
                    },
                    "isHosted": false,
                    "pillarId": "pillar/arts",
                    "pillarName": "Arts"
                }
            ]
        }
    }"#;

    #[tokio::test]
    pub async fn make_capi_request_success() {
        let server = MockServer::start();
        let capi_mock = server.mock(|when, then| {
            when.path("/search");
            then.body(SUCCESS_RESPONSE).header("Content-Type", "application/json").status(200);
        });

        let http_client = Client::builder().build().unwrap();
        let response = make_capi_request(
            &http_client, 
            String::from("some-key-here"), 
            String::from("hello/tags"), 
            1, 
            5, 
            Some(Duration::from_millis(10)),
            None,
            Some(server.base_url())).await;

        print!("{:?}", &response);
        assert!(response.is_ok());
        let returned_content = response.ok().unwrap();

        assert_eq!(returned_content.response.currentPage, 1);
        assert_eq!(returned_content.response.results.len(), 1);
        assert_eq!(returned_content.response.results[0].blocks.body.len(), 1);
        capi_mock.assert_hits(1);
    }

    #[tokio::test]
    pub async fn make_capi_request_nonretryable_failure() {
        let server = MockServer::start();
        let capi_mock = server.mock(|when, then| {
            when.path("/search");
            then.status(404);
        });

        let http_client = Client::builder().build().unwrap();
        let response = make_capi_request(
            &http_client, 
            String::from("some-key-here"), 
            String::from("hello/tags"), 
            1, 
            5, 
            Some(Duration::from_millis(10)),
            None,
            Some(server.base_url())).await;

        print!("{:?}", &response);
        assert!(response.is_err());
        let err_response = response.err().unwrap();
        let returned_content_opt = err_response.downcast_ref::<CapiError>();
        assert!(returned_content_opt.is_some());

        let returned_content = returned_content_opt.unwrap();
        assert_eq!(returned_content.code, 404);
        capi_mock.assert();
    }

    #[tokio::test]
    pub async fn make_capi_request_retryable_failure() {
        let server = MockServer::start();
        let capi_mock = server.mock(|when, then| {
            when.path("/search");
            then.status(503);
        });

        let http_client = Client::builder().build().unwrap();
        let response = make_capi_request(
            &http_client, 
            String::from("some-key-here"), 
            String::from("hello/tags"), 
            1, 
            5, 
            Some(Duration::from_millis(1)),
            Some(10),
            Some(server.base_url())).await;

        print!("{:?}", &response);
        assert!(response.is_err());
        let err_response = response.err().unwrap();
        let returned_content_opt = err_response.downcast_ref::<CapiError>();
        assert!(returned_content_opt.is_some());

        let returned_content = returned_content_opt.unwrap();
        //we should get the 503 response come back to us, but have tried the request 10 times instead of 1
        assert_eq!(returned_content.code, 503);
        capi_mock.assert_hits(10);
    }
}