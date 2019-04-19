//! This crate provides logging facilities to evaluate performances
//! of code parallelized with the rayon parallel computing library.
//! It also doubles down as a debugging tool.
//!
//! Ideally using it should be as easy as adding
//! `extern crate rayon_logs as rayon;`
//! at top of your main file (replacing `extern crate rayon`).
//!
//! However there are currently limitations because we do not
//! currently log all parts of rayon.
//!
//! - the global `ThreadPool` is not logged so it is *required* to use a `ThreadPoolBuilder`.
//! - not all of rayon's traits are implemented. In particular no `IndexedParallelIterator` (no zip),
//! no `FromParallelIterator`  (no  collect)...
//! - `par_sort` is logged but it is not directly rayon's `par_sort` but a copy-pasted version of
//! it (as a demonstration). so the algorithm is hard-coded into rayon_logs.
//! - you should not mix logged and not logged computations.
//! - each call to `ThreadPool::install` generates a json file which can then be converted to svg
//! using `json2svg`.
//! - each log generates an overhead of around 1 micro seconds. This is due to thread_local being
//! very slow.
//!
//! With this being said, here is a small example:
//!
//! Example:
//! ```
//! extern crate rayon_logs as rayon; // comment me out to go back to using rayon
//! use rayon::prelude::*;
//! use rayon::ThreadPoolBuilder;
//! let v = vec![1; 100_000];
//! // let's create a logged pool of threads
//! let pool = ThreadPoolBuilder::new().num_threads(2).build().expect("failed creating pool");
//! // run and log some computations
//! assert_eq!(100_000, pool.install(|| v.par_iter().sum::<u32>()));
//! ```
//!
//! Running this code will create a `log_0.json` file.
//! You can then use `cargo run --bin json2svg -- log_0.json example_sum.svg` to view the log.
//! The resulting file should be viewed in a web browser since it is animated.
//! The bars below the graph represent idle times.
//!
//! <svg viewBox="0 0 1920 1080" version="1.1" xmlns="http://www.w3.org/2000/svg">
//! <line x1="478.22774568207103" y1="386.66666666666674" x2="245.5389422454836" y2="400.0000000000001" stroke="black" stroke-width="2.0"/><line x1="478.22774568207103" y1="386.66666666666674" x2="723.7666879275546" y2="400.0000000000001" stroke="black" stroke-width="2.0"/><line x1="245.5389422454836" y1="466.6666666666668" x2="478.22774568207103" y2="480.00000000000017" stroke="black" stroke-width="2.0"/><line x1="723.7666879275546" y1="466.6666666666668" x2="478.22774568207103" y2="480.00000000000017" stroke="black" stroke-width="2.0"/><line x1="1081.72105381847" y1="386.6666666666668" x2="1020.0304612988932" y2="400.00000000000017" stroke="black" stroke-width="2.0"/><line x1="1081.72105381847" y1="386.6666666666668" x2="1145.296023753221" y2="400.00000000000017" stroke="black" stroke-width="2.0"/><line x1="1020.0304612988932" y1="466.66666666666686" x2="1081.72105381847" y2="480.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1145.296023753221" y1="466.66666666666686" x2="1081.72105381847" y2="480.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1329.8562556290938" y1="386.6666666666668" x2="1269.0548906226725" y2="400.00000000000017" stroke="black" stroke-width="2.0"/><line x1="1329.8562556290938" y1="386.6666666666668" x2="1391.9245299789684" y2="400.00000000000017" stroke="black" stroke-width="2.0"/><line x1="1269.0548906226725" y1="466.66666666666686" x2="1329.8562556290938" y2="480.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1391.9245299789684" y1="466.66666666666686" x2="1329.8562556290938" y2="480.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1204.5906931747659" y1="306.6666666666668" x2="1081.72105381847" y2="320.0000000000001" stroke="black" stroke-width="2.0"/><line x1="1204.5906931747659" y1="306.6666666666668" x2="1329.8562556290938" y2="320.0000000000001" stroke="black" stroke-width="2.0"/><line x1="1081.72105381847" y1="546.6666666666669" x2="1204.5906931747659" y2="560.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1329.8562556290938" y1="546.6666666666669" x2="1204.5906931747659" y2="560.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1569.938982775491" y1="386.6666666666668" x2="1511.6950654928557" y2="400.00000000000017" stroke="black" stroke-width="2.0"/><line x1="1569.938982775491" y1="386.6666666666668" x2="1628.9081532829573" y2="400.00000000000017" stroke="black" stroke-width="2.0"/><line x1="1511.6950654928557" y1="466.66666666666686" x2="1569.938982775491" y2="480.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1628.9081532829573" y1="466.66666666666686" x2="1569.938982775491" y2="480.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1803.5760352827963" y1="386.6666666666668" x2="1745.4013209677826" y2="400.00000000000017" stroke="black" stroke-width="2.0"/><line x1="1803.5760352827963" y1="386.6666666666668" x2="1861.8252856849863" y2="400.00000000000017" stroke="black" stroke-width="2.0"/><line x1="1745.4013209677826" y1="466.66666666666686" x2="1803.5760352827963" y2="480.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1861.8252856849863" y1="466.66666666666686" x2="1803.5760352827963" y2="480.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1686.362947492695" y1="306.6666666666668" x2="1569.938982775491" y2="320.0000000000001" stroke="black" stroke-width="2.0"/><line x1="1686.362947492695" y1="306.6666666666668" x2="1803.5760352827963" y2="320.0000000000001" stroke="black" stroke-width="2.0"/><line x1="1569.938982775491" y1="546.6666666666669" x2="1686.362947492695" y2="560.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1803.5760352827963" y1="546.6666666666669" x2="1686.362947492695" y2="560.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1438.227745682071" y1="226.66666666666674" x2="1204.5906931747659" y2="240.0000000000001" stroke="black" stroke-width="2.0"/><line x1="1438.227745682071" y1="226.66666666666674" x2="1686.362947492695" y2="240.0000000000001" stroke="black" stroke-width="2.0"/><line x1="1204.5906931747659" y1="626.6666666666669" x2="1438.227745682071" y2="640.0000000000002" stroke="black" stroke-width="2.0"/><line x1="1686.362947492695" y1="626.6666666666669" x2="1438.227745682071" y2="640.0000000000002" stroke="black" stroke-width="2.0"/><line x1="960" y1="66.66666666666669" x2="960" y2="80.00000000000001" stroke="black" stroke-width="2.0"/><line x1="960" y1="146.6666666666667" x2="478.22774568207103" y2="320.00000000000006" stroke="black" stroke-width="2.0"/><line x1="960" y1="146.6666666666667" x2="1438.227745682071" y2="160.00000000000006" stroke="black" stroke-width="2.0"/><line x1="478.22774568207103" y1="546.6666666666667" x2="960" y2="720.0000000000001" stroke="black" stroke-width="2.0"/><line x1="1438.227745682071" y1="706.666666666667" x2="960" y2="720.0000000000001" stroke="black" stroke-width="2.0"/><line x1="960" y1="786.6666666666669" x2="960" y2="800.0000000000001" stroke="black" stroke-width="2.0"/><rect x="959.7911407485313" y="0" width="0.41771850293720286" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="959.7911407485313" y="0" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.41771850293720286" begin="0s" dur="0.02582189172724637s" fill="freeze"/>
//! </rect>
//! <rect x="959.9495267441167" y="80.00000000000001" width="0.10094651176662031" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="959.9495267441167" y="80.00000000000001" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.10094651176662031" begin="0.040049959129797516s" dur="0.006240159051495824s" fill="freeze"/>
//! </rect>
//! <rect x="478.198298696586" y="320.00000000000006" width="0.05889397097001003" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="478.198298696586" y="320.00000000000006" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.05889397097001003" begin="0.059937705672077436s" dur="0.003640618577060759s" fill="freeze"/>
//! </rect>
//! <rect x="0" y="400.0000000000001" width="491.0778844909672" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="0" y="400.0000000000001" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="491.0778844909672" begin="0.06867960695200409s" dur="30.35671121534511s" fill="freeze"/>
//! </rect>
//! <rect x="491.0778844909672" y="400.0000000000001" width="465.3776068731748" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="491.0778844909672" y="400.0000000000001" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="465.3776068731748" begin="30.435520827713876s" dur="28.768010256827658s" fill="freeze"/>
//! </rect>
//! <rect x="478.1687751591457" y="480.00000000000017" width="0.11794104585068732" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="478.1687751591457" y="480.00000000000017" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.11794104585068732" begin="59.206105386758935s" dur="0.007290701500509024s" fill="freeze"/>
//! </rect>
//! <rect x="1438.1600737535562" y="160.00000000000006" width="0.13534385702986707" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1438.1600737535562" y="160.00000000000006" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.13534385702986707" begin="0.09356768316901651s" dur="0.008366482206555574s" fill="freeze"/>
//! </rect>
//! <rect x="1204.559638598219" y="240.0000000000001" width="0.06210915309402271" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1204.559638598219" y="240.0000000000001" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.06210915309402271" begin="0.11360370447136736s" dur="0.0038393698511983913s" fill="freeze"/>
//! </rect>
//! <rect x="1081.6917344195772" y="320.0000000000001" width="0.05863879778556458" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1081.6917344195772" y="320.0000000000001" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.05863879778556458" begin="0.12349079166418228s" dur="0.003624844666414915s" fill="freeze"/>
//! </rect>
//! <rect x="956.4554913641421" y="400.00000000000017" width="127.1499398695023" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="956.4554913641421" y="400.00000000000017" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="127.1499398695023" begin="0.1318099521388003s" dur="7.859963007024754s" fill="freeze"/>
//! </rect>
//! <rect x="1083.6054312336444" y="400.00000000000017" width="123.38118503915366" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1083.6054312336444" y="400.00000000000017" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="123.38118503915366" begin="8.001852488066248s" dur="7.626991811132028s" fill="freeze"/>
//! </rect>
//! <rect x="1081.6611646720808" y="480.0000000000002" width="0.11977829277869458" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1081.6611646720808" y="480.0000000000002" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.11977829277869458" begin="15.633617484559707s" dur="0.0074042736571591s" fill="freeze"/>
//! </rect>
//! <rect x="1329.820505865953" y="320.0000000000001" width="0.0714995262816153" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1329.820505865953" y="320.0000000000001" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.0714995262816153" begin="15.648567997069838s" dur="0.004419849762965444s" fill="freeze"/>
//! </rect>
//! <rect x="1206.986616272798" y="400.00000000000017" width="124.13654869974908" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1206.986616272798" y="400.00000000000017" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="124.13654869974908" begin="15.6601365831375s" dur="7.673685741425854s" fill="freeze"/>
//! </rect>
//! <rect x="1331.123164972547" y="400.00000000000017" width="121.60273001284266" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1331.123164972547" y="400.00000000000017" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="121.60273001284266" begin="23.34215094938436s" dur="7.517053963494754s" fill="freeze"/>
//! </rect>
//! <rect x="1329.8028478815895" y="480.0000000000002" width="0.10681549500886567" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1329.8028478815895" y="480.0000000000002" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.10681549500886567" begin="30.862388088047446s" dur="0.006602958996350232s" fill="freeze"/>
//! </rect>
//! <rect x="1204.5503758116236" y="560.0000000000002" width="0.08063472628476243" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1204.5503758116236" y="560.0000000000002" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.08063472628476243" begin="30.87076087981826s" dur="0.004984555764086654s" fill="freeze"/>
//! </rect>
//! <rect x="1686.3227321988263" y="240.0000000000001" width="0.08043058773720607" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1686.3227321988263" y="240.0000000000001" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.08043058773720607" begin="30.890818984595516s" dur="0.004971936635569979s" fill="freeze"/>
//! </rect>
//! <rect x="1569.909357168777" y="320.0000000000001" width="0.05925121342823366" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1569.909357168777" y="320.0000000000001" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.05925121342823366" begin="30.907441531634106s" dur="0.0036627020519649405s" fill="freeze"/>
//! </rect>
//! <rect x="1452.7258949853897" y="400.00000000000017" width="117.9383410149322" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1452.7258949853897" y="400.00000000000017" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="117.9383410149322" begin="30.916095099014413s" dur="7.290534297056178s" fill="freeze"/>
//! </rect>
//! <rect x="1570.664236000322" y="400.00000000000017" width="116.48783456527049" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1570.664236000322" y="400.00000000000017" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="116.48783456527049" begin="38.21558897731743s" dur="7.200869079380944s" fill="freeze"/>
//! </rect>
//! <rect x="1569.9035647374901" y="480.0000000000002" width="0.07083607600205712" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1569.9035647374901" y="480.0000000000002" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.07083607600205712" begin="45.4186127728926s" dur="0.004378837595286251s" fill="freeze"/>
//! </rect>
//! <rect x="1803.5407703487058" y="320.0000000000001" width="0.07052986818072259" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1803.5407703487058" y="320.0000000000001" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.07052986818072259" begin="45.43128237792334s" dur="0.0043599089025112385s" fill="freeze"/>
//! </rect>
//! <rect x="1687.1520705655923" y="400.00000000000017" width="116.4985008043803" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1687.1520705655923" y="400.00000000000017" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="116.4985008043803" begin="45.443746922115686s" dur="7.20152842884594s" fill="freeze"/>
//! </rect>
//! <rect x="1803.6505713699728" y="400.00000000000017" width="116.34942863002728" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1803.6505713699728" y="400.00000000000017" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="116.34942863002728" begin="52.650764671866376s" dur="7.192313310246638s" fill="freeze"/>
//! </rect>
//! <rect x="1803.5279861721651" y="480.0000000000002" width="0.09609822126215674" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1803.5279861721651" y="480.0000000000002" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.09609822126215674" begin="59.84476894533425s" dur="0.005940454749224791s" fill="freeze"/>
//! </rect>
//! <rect x="1686.318419772009" y="560.0000000000002" width="0.0890554413714623" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1686.318419772009" y="560.0000000000002" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.0890554413714623" begin="59.852334112879994s" dur="0.005505094815399501s" fill="freeze"/>
//! </rect>
//! <rect x="1438.1974311077588" y="640.0000000000002" width="0.06062914862423909" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="1438.1974311077588" y="640.0000000000002" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.06062914862423909" begin="59.85939136050295s" dur="0.003747881169452497s" fill="freeze"/>
//! </rect>
//! <rect x="959.8472278144725" y="720.0000000000001" width="0.3055443710549827" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="959.8472278144725" y="720.0000000000001" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.3055443710549827" begin="59.97461346820658s" dur="0.018887680607333417s" fill="freeze"/>
//! </rect>
//! <rect x="959.9674909363016" y="800.0000000000001" width="0.06501812739670085" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="959.9674909363016" y="800.0000000000001" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.06501812739670085" begin="59.99598080756744s" dur="0.004019192432561011s" fill="freeze"/>
//! </rect>
//! <rect x="192.00000000000003" y="933.3333333333335" width="0.23016621236979656" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="192.00000000000003" y="933.3333333333335" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.23016621236979656" begin="0.02582189172724637s" dur="0.014228067402551146s" fill="freeze"/>
//! </rect>
//! <rect x="192.23016621236982" y="933.3333333333335" width="0.22077583918220398" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="192.23016621236982" y="933.3333333333335" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.22077583918220398" begin="0.04629011818129334s" dur="0.013647587490784093s" fill="freeze"/>
//! </rect>
//! <rect x="192.450942051552" y="933.3333333333335" width="0.08252300784965877" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="192.450942051552" y="933.3333333333335" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.08252300784965877" begin="0.06357832424913819s" dur="0.005101282702865899s" fill="freeze"/>
//! </rect>
//! <rect x="192.00000000000003" y="1013.3333333333335" width="1.5136362954935245" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="192.00000000000003" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="1.5136362954935245" begin="0s" dur="0.09356768316901651s" fill="freeze"/>
//! </rect>
//! <rect x="193.51363629549354" y="1013.3333333333335" width="0.18877712185274445" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="193.51363629549354" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.18877712185274445" begin="0.10193416537557208s" dur="0.011669539095795274s" fill="freeze"/>
//! </rect>
//! <rect x="193.7024134173463" y="1013.3333333333335" width="0.0978333989163858" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="193.7024134173463" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.0978333989163858" begin="0.11744307432256575s" dur="0.0060477173416165295s" fill="freeze"/>
//! </rect>
//! <rect x="193.80024681626267" y="1013.3333333333335" width="0.07593953969096613" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="193.80024681626267" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.07593953969096613" begin="0.1271156363305972s" dur="0.004694315808203128s" fill="freeze"/>
//! </rect>
//! <rect x="193.87618635595365" y="1013.3333333333335" width="0.16305566486064302" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="193.87618635595365" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.16305566486064302" begin="7.991772959163554s" dur="0.010079528902694215s" fill="freeze"/>
//! </rect>
//! <rect x="194.03924202081427" y="1013.3333333333335" width="0.07721540561319339" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="194.03924202081427" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.07721540561319339" begin="15.628844299198276s" dur="0.004773185361432347s" fill="freeze"/>
//! </rect>
//! <rect x="194.11645742642747" y="1013.3333333333335" width="0.12207485143870363" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="194.11645742642747" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.12207485143870363" begin="15.641021758216867s" dur="0.007546238852971694s" fill="freeze"/>
//! </rect>
//! <rect x="194.23853227786617" y="1013.3333333333335" width="0.11564448719067827" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="194.23853227786617" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.11564448719067827" begin="15.652987846832804s" dur="0.00714873630469643s" fill="freeze"/>
//! </rect>
//! <rect x="194.35417676505685" y="1013.3333333333335" width="0.13473144138719798" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="194.35417676505685" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.13473144138719798" begin="23.333822324563354s" dur="0.00832862482100555s" fill="freeze"/>
//! </rect>
//! <rect x="192.53346505940166" y="933.3333333333335" width="0.16387221905086846" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="192.53346505940166" y="933.3333333333335" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.16387221905086846" begin="30.425390822297118s" dur="0.010130005416760915s" fill="freeze"/>
//! </rect>
//! <rect x="194.48890820644405" y="1013.3333333333335" width="0.05149394862109195" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="194.48890820644405" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.05149394862109195" begin="30.859204912879115s" dur="0.0031831751683312876s" fill="freeze"/>
//! </rect>
//! <rect x="194.54040215506515" y="1013.3333333333335" width="0.02863043129477957" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="194.54040215506515" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.02863043129477957" begin="30.868991047043796s" dur="0.0017698327744636792s" fill="freeze"/>
//! </rect>
//! <rect x="194.56903258635992" y="1013.3333333333335" width="0.24384349505607272" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="194.56903258635992" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.24384349505607272" begin="30.875745435582346s" dur="0.015073549013168376s" fill="freeze"/>
//! </rect>
//! <rect x="194.812876081416" y="1013.3333333333335" width="0.1884709140314099" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="194.812876081416" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.1884709140314099" begin="30.895790921231086s" dur="0.011650610403020262s" fill="freeze"/>
//! </rect>
//! <rect x="195.0013469954474" y="1013.3333333333335" width="0.0807367955585406" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.0013469954474" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.0807367955585406" begin="30.91110423368607s" dur="0.0049908653283449915s" fill="freeze"/>
//! </rect>
//! <rect x="195.08208379100594" y="1013.3333333333335" width="0.144938368765016" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.08208379100594" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.144938368765016" begin="38.20662939607059s" dur="0.008959581246839302s" fill="freeze"/>
//! </rect>
//! <rect x="195.22702215977097" y="1013.3333333333335" width="0.03485665699524857" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.22702215977097" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.03485665699524857" begin="45.416458056698374s" dur="0.002154716194222269s" fill="freeze"/>
//! </rect>
//! <rect x="195.2618788167662" y="1013.3333333333335" width="0.1341190257445289" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.2618788167662" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.1341190257445289" begin="45.422991610487884s" dur="0.008290767435455523s" fill="freeze"/>
//! </rect>
//! <rect x="195.39599784251075" y="1013.3333333333335" width="0.1311079821680726" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.39599784251075" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.1311079821680726" begin="45.43564228682585s" dur="0.008104635289834567s" fill="freeze"/>
//! </rect>
//! <rect x="195.5271058246788" y="1013.3333333333335" width="0.08880026818701685" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.5271058246788" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.08880026818701685" begin="52.645275350961626s" dur="0.005489320904753657s" fill="freeze"/>
//! </rect>
//! <rect x="192.69733727845255" y="933.3333333333335" width="0.041644263701497555" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="192.69733727845255" y="933.3333333333335" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.041644263701497555" begin="59.20353108454154s" dur="0.0025743022174017152s" fill="freeze"/>
//! </rect>
//! <rect x="195.61590609286583" y="1013.3333333333335" width="0.027354565372552316" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.61590609286583" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.027354565372552316" begin="59.843077982113016s" dur="0.00169096322123446s" fill="freeze"/>
//! </rect>
//! <rect x="195.64326065823838" y="1013.3333333333335" width="0.026282837997881423" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.64326065823838" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.026282837997881423" begin="59.850709400083474s" dur="0.0016247127965219159s" fill="freeze"/>
//! </rect>
//! <rect x="195.66954349623626" y="1013.3333333333335" width="0.02510904134943235" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.66954349623626" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.02510904134943235" begin="59.8578392076954s" dur="0.001552152807551034s" fill="freeze"/>
//! </rect>
//! <rect x="192.73898154215405" y="933.3333333333335" width="12.31414753496856" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="192.73898154215405" y="933.3333333333335" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="12.31414753496856" begin="59.213396088259444s" dur="0.7612173799471321s" fill="freeze"/>
//! </rect>
//! <rect x="205.0531290771226" y="933.3333333333335" width="0.04011322459482485" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="205.0531290771226" y="933.3333333333335" width="0" height="66.66666666666669" fill="rgba(255,0,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="0.04011322459482485" begin="59.99350114881391s" dur="0.002479658753526652s" fill="freeze"/>
//! </rect>
//! <rect x="195.69465253758568" y="1013.3333333333335" width="2.2139846175225086" height="66.66666666666669" fill="black"/>
//! <rect class="task6261051670931369750" x="195.69465253758568" y="1013.3333333333335" width="0" height="66.66666666666669" fill="rgba(0,255,0,1)">
//! <animate attributeType="XML" attributeName="width" from="0" to="2.2139846175225086" begin="59.8631392416724s" dur="0.13686075832759953s" fill="freeze"/>
//! </rect>
//! </svg>
#![type_length_limit = "2097152"] // it seems we have types with long names
#![deny(missing_docs)]
#![warn(clippy::all)]

mod pool; // this comes first because it exports the logs macro

mod iterator;
mod storage;
pub use crate::iterator::Logged;
pub use crate::pool::{join, join_context, subgraph, subgraph_perf, ThreadPool};
mod builder;
pub mod prelude;
pub use crate::builder::ThreadPoolBuilder;
mod scope;
pub use crate::scope::{scope, Scope};
mod fork_join_graph;
mod stats;
pub use crate::fork_join_graph::visualisation;
pub(crate) mod compare;
mod log;
pub use crate::log::RunLog;
mod rayon_algorithms;
pub(crate) mod svg;
pub use crate::compare::Comparator;
pub(crate) mod raw_events;
/// We re-export rayon's `current_num_threads`.
pub use rayon::current_num_threads;
